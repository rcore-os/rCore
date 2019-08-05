use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::fmt;

use core::str;
use log::*;
use rcore_memory::{Page, PAGE_SIZE};
use rcore_thread::Tid;
use spin::RwLock;
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
    ElfFile,
};

use crate::arch::interrupt::{Context, TrapFrame};
use crate::fs::{FileHandle, FileLike, OpenOptions, FOLLOW_MAX_DEPTH};
use crate::memory::{
    ByFrame, Delay, File, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet, Read,
};
use crate::sync::{Condvar, SpinNoIrqLock as Mutex};

use super::abi::{self, ProcInitInfo};
use crate::processor;
use core::mem::MaybeUninit;
use rcore_fs::vfs::INode;

pub struct Thread {
    context: Context,
    kstack: KernelStack,
    /// Kernel performs futex wake when thread exits.
    /// Ref: [http://man7.org/linux/man-pages/man2/set_tid_address.2.html]
    pub clear_child_tid: usize,
    // This is same as `proc.vm`
    pub vm: Arc<Mutex<MemorySet>>,
    pub proc: Arc<Mutex<Process>>,
}

/// Pid type
/// For strong type separation
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(usize);

impl Pid {
    pub fn get(&self) -> usize {
        self.0
    }

    /// Return whether this pid represents the init process
    pub fn is_init(&self) -> bool {
        self.0 == 0
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Process {
    // resources
    pub vm: Arc<Mutex<MemorySet>>,
    pub files: BTreeMap<usize, FileLike>,
    pub cwd: String,
    pub exec_path: String,
    futexes: BTreeMap<usize, Arc<Condvar>>,

    // relationship
    pub pid: Pid, // i.e. tgid, usually the tid of first thread
    pub parent: Weak<Mutex<Process>>,
    pub children: Vec<Weak<Mutex<Process>>>,
    pub threads: Vec<Tid>, // threads in the same process

    // for waiting child
    pub child_exit: Arc<Condvar>, // notified when the a child process is going to terminate
    pub child_exit_code: BTreeMap<usize, usize>, // child process store its exit code here
}

lazy_static! {
    /// Records the mapping between pid and Process struct.
    pub static ref PROCESSES: RwLock<BTreeMap<usize, Weak<Mutex<Process>>>> =
        RwLock::new(BTreeMap::new());
}

/// Let `rcore_thread` can switch between our `Thread`
impl rcore_thread::Context for Thread {
    unsafe fn switch_to(&mut self, target: &mut dyn rcore_thread::Context) {
        use core::mem::transmute;
        let (target, _): (&mut Thread, *const ()) = transmute(target);
        self.context.switch(&mut target.context);
    }

    fn set_tid(&mut self, tid: Tid) {
        let mut proc = self.proc.lock();
        // add it to threads
        proc.threads.push(tid);
    }
}

impl Thread {
    /// Make a struct for the init thread
    pub unsafe fn new_init() -> Box<Thread> {
        Box::new(Thread {
            context: Context::null(),
            // safety: other fields will never be used
            ..core::mem::MaybeUninit::zeroed().assume_init()
        })
    }

    /// Make a new kernel thread starting from `entry` with `arg`
    pub fn new_kernel(entry: extern "C" fn(usize) -> !, arg: usize) -> Box<Thread> {
        let vm = MemorySet::new();
        let vm_token = vm.token();
        let vm = Arc::new(Mutex::new(vm));
        let kstack = KernelStack::new();
        Box::new(Thread {
            context: unsafe { Context::new_kernel_thread(entry, arg, kstack.top(), vm_token) },
            kstack,
            clear_child_tid: 0,
            vm: vm.clone(),
            // TODO: kernel thread should not have a process
            proc: Process {
                vm,
                files: BTreeMap::default(),
                cwd: String::from("/"),
                exec_path: String::new(),
                futexes: BTreeMap::default(),
                pid: Pid(0),
                parent: Weak::new(),
                children: Vec::new(),
                threads: Vec::new(),
                child_exit: Arc::new(Condvar::new()),
                child_exit_code: BTreeMap::new(),
            }
            .add_to_table(),
        })
    }

    /// Construct virtual memory of a new user process from ELF `data`.
    /// Return `(MemorySet, entry_point, ustack_top)`
    pub fn new_user_vm(
        inode: &Arc<dyn INode>,
        exec_path: &str,
        mut args: Vec<String>,
        envs: Vec<String>,
    ) -> Result<(MemorySet, usize, usize), &'static str> {
        // Read ELF header
        // 0x3c0: magic number from ld-musl.so
        let mut data: [u8; 0x3c0] = unsafe { MaybeUninit::zeroed().assume_init() };
        inode
            .read_at(0, &mut data)
            .map_err(|_| "failed to read from INode")?;

        // Parse ELF
        let elf = ElfFile::new(&data)?;

        // Check ELF type
        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {}
            header::Type::SharedObject => {}
            _ => return Err("ELF is not executable or shared object"),
        }

        // Check ELF arch
        match elf.header.pt2.machine().as_machine() {
            #[cfg(target_arch = "x86_64")]
            header::Machine::X86_64 => {}
            #[cfg(target_arch = "aarch64")]
            header::Machine::AArch64 => {}
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            header::Machine::Other(243) => {}
            #[cfg(target_arch = "mips")]
            header::Machine::Mips => {}
            _ => return Err("invalid ELF arch"),
        }

        let mut auxv = {
            let mut map = BTreeMap::new();
            if let Some(phdr_vaddr) = elf.get_phdr_vaddr() {
                map.insert(abi::AT_PHDR, phdr_vaddr as usize);
            }
            map.insert(abi::AT_PHENT, elf.header.pt2.ph_entry_size() as usize);
            map.insert(abi::AT_PHNUM, elf.header.pt2.ph_count() as usize);
            map.insert(abi::AT_PAGESZ, PAGE_SIZE);
            map
        };
        let mut entry_addr = elf.header.pt2.entry_point() as usize;
        // Make page table
        let (mut vm, bias) = elf.make_memory_set(inode);

        // Check interpreter (for dynamic link)
        // When interpreter is used, map both dynamic linker and executable
        if let Ok(loader_path) = elf.get_interpreter() {
            info!("Handling interpreter... offset={:x}", bias);
            // assuming absolute path
            let interp_inode = crate::fs::ROOT_INODE
                .lookup_follow(loader_path, FOLLOW_MAX_DEPTH)
                .map_err(|_| "interpreter not found")?;
            // load loader by bias and set aux vector.
            let mut interp_data: [u8; 0x3c0] = unsafe { MaybeUninit::zeroed().assume_init() };
            interp_inode
                .read_at(0, &mut interp_data)
                .map_err(|_| "failed to read from INode")?;
            let elf_interp = ElfFile::new(&interp_data)?;
            elf_interp.append_as_interpreter(&interp_inode, &mut vm, bias);
            debug!("entry point: {:x}", elf.header.pt2.entry_point() as usize);
            auxv.insert(abi::AT_ENTRY, elf.header.pt2.entry_point() as usize);
            auxv.insert(abi::AT_BASE, bias);
            entry_addr = elf_interp.header.pt2.entry_point() as usize + bias;
        }

        // User stack
        use crate::consts::{USER_STACK_OFFSET, USER_STACK_SIZE};
        let mut ustack_top = {
            let ustack_buttom = USER_STACK_OFFSET;
            let ustack_top = USER_STACK_OFFSET + USER_STACK_SIZE;
            vm.push(
                ustack_buttom,
                ustack_top - PAGE_SIZE * 4,
                MemoryAttr::default().user(),
                Delay::new(GlobalFrameAlloc),
                "user_stack_delay",
            );
            // We are going to write init info now. So map the last 4 pages eagerly.
            vm.push(
                ustack_top - PAGE_SIZE * 4,
                ustack_top,
                MemoryAttr::default().user(),
                ByFrame::new(GlobalFrameAlloc),
                "user_stack",
            );
            ustack_top
        };

        // Make init info
        let init_info = ProcInitInfo { args, envs, auxv };
        unsafe {
            vm.with(|| ustack_top = init_info.push_at(ustack_top));
        }

        Ok((vm, entry_addr, ustack_top))
    }

    /// Make a new user process from ELF `data`
    pub fn new_user(
        inode: &Arc<dyn INode>,
        exec_path: &str,
        args: Vec<String>,
        envs: Vec<String>,
    ) -> Box<Thread> {
        let (vm, entry_addr, ustack_top) = Self::new_user_vm(inode, exec_path, args, envs).unwrap();

        let vm_token = vm.token();
        let vm = Arc::new(Mutex::new(vm));
        let kstack = KernelStack::new();

        let mut files = BTreeMap::new();
        files.insert(
            0,
            FileLike::File(FileHandle::new(
                crate::fs::STDIN.clone(),
                OpenOptions {
                    read: true,
                    write: false,
                    append: false,
                    nonblock: false,
                },
                String::from("stdin"),
            )),
        );
        files.insert(
            1,
            FileLike::File(FileHandle::new(
                crate::fs::STDOUT.clone(),
                OpenOptions {
                    read: false,
                    write: true,
                    append: false,
                    nonblock: false,
                },
                String::from("stdout"),
            )),
        );
        files.insert(
            2,
            FileLike::File(FileHandle::new(
                crate::fs::STDOUT.clone(),
                OpenOptions {
                    read: false,
                    write: true,
                    append: false,
                    nonblock: false,
                },
                String::from("stderr"),
            )),
        );

        Box::new(Thread {
            context: unsafe {
                Context::new_user_thread(entry_addr, ustack_top, kstack.top(), vm_token)
            },
            kstack,
            clear_child_tid: 0,
            vm: vm.clone(),
            proc: Process {
                vm,
                files,
                cwd: String::from("/"),
                exec_path: String::from(exec_path),
                futexes: BTreeMap::default(),
                pid: Pid(0),
                parent: Weak::new(),
                children: Vec::new(),
                threads: Vec::new(),
                child_exit: Arc::new(Condvar::new()),
                child_exit_code: BTreeMap::new(),
            }
            .add_to_table(),
        })
    }

    /// Fork a new process from current one
    pub fn fork(&self, tf: &TrapFrame) -> Box<Thread> {
        let kstack = KernelStack::new();
        let vm = self.vm.lock().clone();
        let vm_token = vm.token();
        let vm = Arc::new(Mutex::new(vm));
        let context = unsafe { Context::new_fork(tf, kstack.top(), vm_token) };

        let mut proc = self.proc.lock();
        let new_proc = Process {
            vm: vm.clone(),
            files: proc.files.clone(),
            cwd: proc.cwd.clone(),
            exec_path: proc.exec_path.clone(),
            futexes: BTreeMap::default(),
            pid: Pid(0),
            parent: Arc::downgrade(&self.proc),
            children: Vec::new(),
            threads: Vec::new(),
            child_exit: Arc::new(Condvar::new()),
            child_exit_code: BTreeMap::new(),
        }
        .add_to_table();
        // link to parent
        proc.children.push(Arc::downgrade(&new_proc));

        Box::new(Thread {
            context,
            kstack,
            clear_child_tid: 0,
            vm,
            proc: new_proc,
        })
    }

    /// Create a new thread in the same process.
    pub fn clone(
        &self,
        tf: &TrapFrame,
        stack_top: usize,
        tls: usize,
        clear_child_tid: usize,
    ) -> Box<Thread> {
        let kstack = KernelStack::new();
        let vm_token = self.vm.lock().token();
        Box::new(Thread {
            context: unsafe { Context::new_clone(tf, stack_top, kstack.top(), vm_token, tls) },
            kstack,
            clear_child_tid,
            vm: self.vm.clone(),
            proc: self.proc.clone(),
        })
    }
}

impl Process {
    /// Assign a pid and put itself to global process table.
    fn add_to_table(mut self) -> Arc<Mutex<Self>> {
        let mut process_table = PROCESSES.write();

        // assign pid
        let pid = (0..)
            .find(|i| match process_table.get(i) {
                Some(p) if p.upgrade().is_some() => false,
                _ => true,
            })
            .unwrap();
        self.pid = Pid(pid);

        // put to process table
        let self_ref = Arc::new(Mutex::new(self));
        process_table.insert(pid, Arc::downgrade(&self_ref));

        self_ref
    }
    fn get_free_fd(&self) -> usize {
        (0..).find(|i| !self.files.contains_key(i)).unwrap()
    }
    /// Add a file to the process, return its fd.
    pub fn add_file(&mut self, file_like: FileLike) -> usize {
        let fd = self.get_free_fd();
        self.files.insert(fd, file_like);
        fd
    }
    pub fn get_futex(&mut self, uaddr: usize) -> Arc<Condvar> {
        if !self.futexes.contains_key(&uaddr) {
            self.futexes.insert(uaddr, Arc::new(Condvar::new()));
        }
        self.futexes.get(&uaddr).unwrap().clone()
    }
    /// Exit the process.
    /// Kill all threads and notify parent with the exit code.
    pub fn exit(&mut self, exit_code: usize) {
        // quit all threads
        for tid in self.threads.iter() {
            processor().manager().exit(*tid, 1);
        }
        // notify parent and fill exit code
        if let Some(parent) = self.parent.upgrade() {
            let mut parent = parent.lock();
            parent.child_exit_code.insert(self.pid.get(), exit_code);
            parent.child_exit.notify_one();
        }
    }
}

trait ToMemoryAttr {
    fn to_attr(&self) -> MemoryAttr;
}

impl ToMemoryAttr for Flags {
    fn to_attr(&self) -> MemoryAttr {
        let mut flags = MemoryAttr::default().user();
        if self.is_execute() {
            flags = flags.execute();
        }
        if !self.is_write() {
            flags = flags.readonly();
        }
        flags
    }
}

/// Helper functions to process ELF file
trait ElfExt {
    /// Generate a MemorySet according to the ELF file.
    fn make_memory_set(&self, inode: &Arc<dyn INode>) -> (MemorySet, usize);

    /// Get interpreter string if it has.
    fn get_interpreter(&self) -> Result<&str, &str>;

    /// Append current ELF file as interpreter into given memory set.
    /// This will insert the interpreter it a place which is "good enough" (since ld.so should be PIC).
    fn append_as_interpreter(
        &self,
        inode: &Arc<dyn INode>,
        memory_set: &mut MemorySet,
        bias: usize,
    );

    /// Get virtual address of PHDR section if it has.
    fn get_phdr_vaddr(&self) -> Option<u64>;
}

impl ElfExt for ElfFile<'_> {
    fn make_memory_set(&self, inode: &Arc<dyn INode>) -> (MemorySet, usize) {
        debug!("creating MemorySet from ELF");
        let mut ms = MemorySet::new();
        let mut farthest_memory: usize = 0;
        for ph in self.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            ms.push(
                ph.virtual_addr() as usize,
                ph.virtual_addr() as usize + ph.mem_size() as usize,
                ph.flags().to_attr(),
                File {
                    file: INodeForMap(inode.clone()),
                    mem_start: ph.virtual_addr() as usize,
                    file_start: ph.offset() as usize,
                    file_end: ph.offset() as usize + ph.file_size() as usize,
                    allocator: GlobalFrameAlloc,
                },
                "elf",
            );
            if ph.virtual_addr() as usize + ph.mem_size() as usize > farthest_memory {
                farthest_memory = ph.virtual_addr() as usize + ph.mem_size() as usize;
            }
        }
        (
            ms,
            (Page::of_addr(farthest_memory + PAGE_SIZE)).start_address(),
        )
    }
    fn append_as_interpreter(&self, inode: &Arc<dyn INode>, ms: &mut MemorySet, bias: usize) {
        debug!("inserting interpreter from ELF");

        for ph in self.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            ms.push(
                ph.virtual_addr() as usize + bias,
                ph.virtual_addr() as usize + ph.mem_size() as usize + bias,
                ph.flags().to_attr(),
                File {
                    file: INodeForMap(inode.clone()),
                    mem_start: ph.virtual_addr() as usize + bias,
                    file_start: ph.offset() as usize,
                    file_end: ph.offset() as usize + ph.file_size() as usize,
                    allocator: GlobalFrameAlloc,
                },
                "elf-interp",
            )
        }
    }
    fn get_interpreter(&self) -> Result<&str, &str> {
        let header = self
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Interp))
            .next()
            .ok_or("no interp header")?;
        let mut data = match header.get_data(self)? {
            SegmentData::Undefined(data) => data,
            _ => unreachable!(),
        };
        // skip NULL
        while let Some(0) = data.last() {
            data = &data[..data.len() - 1];
        }
        let path = str::from_utf8(data).map_err(|_| "failed to convert to utf8")?;
        Ok(path)
    }

    fn get_phdr_vaddr(&self) -> Option<u64> {
        if let Some(phdr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Phdr))
        {
            // if phdr exists in program header, use it
            Some(phdr.virtual_addr())
        } else if let Some(elf_addr) = self
            .program_iter()
            .find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0)
        {
            // otherwise, check if elf is loaded from the beginning, then phdr can be inferred.
            Some(elf_addr.virtual_addr() + self.header.pt2.ph_offset())
        } else {
            warn!("elf: no phdr found, tls might not work");
            None
        }
    }
}

#[derive(Clone)]
pub struct INodeForMap(pub Arc<dyn INode>);

impl Read for INodeForMap {
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        self.0.read_at(offset, buf).unwrap()
    }
}
