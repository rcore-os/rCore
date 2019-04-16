use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::fmt;

use core::str;
use log::*;
use rcore_memory::PAGE_SIZE;
use rcore_thread::Tid;
use spin::RwLock;
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
    ElfFile,
};

use crate::arch::interrupt::{Context, TrapFrame};
use crate::fs::{FileHandle, FileLike, INodeExt, OpenOptions, FOLLOW_MAX_DEPTH};
use crate::memory::{ByFrame, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet};
use crate::net::SOCKETS;
use crate::sync::{Condvar, SpinNoIrqLock as Mutex};

use super::abi::{self, ProcInitInfo};

// TODO: avoid pub
pub struct Thread {
    pub context: Context,
    pub kstack: KernelStack,
    /// Kernel performs futex wake when thread exits.
    /// Ref: [http://man7.org/linux/man-pages/man2/set_tid_address.2.html]
    pub clear_child_tid: usize,
    pub proc: Arc<Mutex<Process>>,
}

/// Pid type
/// For strong type separation
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(Option<usize>);

impl Pid {
    pub fn uninitialized() -> Self {
        Pid(None)
    }

    /// Return if it was uninitialized before this call
    /// When returning true, it usually means this is the first thread
    pub fn set_if_uninitialized(&mut self, tid: Tid) -> bool {
        if self.0 == None {
            self.0 = Some(tid as usize);
            true
        } else {
            false
        }
    }

    pub fn get(&self) -> usize {
        self.0.unwrap()
    }

    /// Return whether this pid represents the init process
    pub fn is_init(&self) -> bool {
        self.0 == Some(0)
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.0 {
            Some(pid) => write!(f, "{}", pid),
            None => write!(f, "None"),
        }
    }
}

pub struct Process {
    // resources
    pub vm: MemorySet,
    pub files: BTreeMap<usize, FileLike>,
    pub cwd: String,
    futexes: BTreeMap<usize, Arc<Condvar>>,

    // relationship
    pub pid: Pid, // i.e. tgid, usually the tid of first thread
    pub parent: Option<Arc<Mutex<Process>>>,
    pub children: Vec<Weak<Mutex<Process>>>,
    pub threads: Vec<Tid>, // threads in the same process

    // for waiting child
    pub child_exit: Arc<Condvar>, // notified when the a child process is going to terminate
    pub child_exit_code: BTreeMap<usize, usize>, // child process store its exit code here
}

/// Records the mapping between pid and Process struct.
lazy_static! {
    pub static ref PROCESSES: RwLock<BTreeMap<usize, Weak<Mutex<Process>>>> =
        RwLock::new(BTreeMap::new());
}

/// Let `rcore_thread` can switch between our `Thread`
impl rcore_thread::Context for Thread {
    unsafe fn switch_to(&mut self, target: &mut rcore_thread::Context) {
        use core::mem::transmute;
        let (target, _): (&mut Thread, *const ()) = transmute(target);
        self.context.switch(&mut target.context);
    }

    fn set_tid(&mut self, tid: Tid) {
        // set pid=tid if unspecified
        let mut proc = self.proc.lock();
        if proc.pid.set_if_uninitialized(tid) {
            // first thread in the process
            // link to its ppid
            if let Some(parent) = &proc.parent {
                let mut parent = parent.lock();
                parent.children.push(Arc::downgrade(&self.proc));
            }
        }
        // add it to threads
        proc.threads.push(tid);
        PROCESSES
            .write()
            .insert(proc.pid.get(), Arc::downgrade(&self.proc));
    }
}

impl Thread {
    /// Make a struct for the init thread
    pub unsafe fn new_init() -> Box<Thread> {
        Box::new(Thread {
            context: Context::null(),
            kstack: KernelStack::new(),
            clear_child_tid: 0,
            // safety: this field will never be used
            proc: core::mem::uninitialized(),
        })
    }

    /// Make a new kernel thread starting from `entry` with `arg`
    pub fn new_kernel(entry: extern "C" fn(usize) -> !, arg: usize) -> Box<Thread> {
        let vm = MemorySet::new();
        let kstack = KernelStack::new();
        Box::new(Thread {
            context: unsafe { Context::new_kernel_thread(entry, arg, kstack.top(), vm.token()) },
            kstack,
            clear_child_tid: 0,
            // TODO: kernel thread should not have a process
            proc: Arc::new(Mutex::new(Process {
                vm,
                files: BTreeMap::default(),
                cwd: String::from("/"),
                futexes: BTreeMap::default(),
                pid: Pid::uninitialized(),
                parent: None,
                children: Vec::new(),
                threads: Vec::new(),
                child_exit: Arc::new(Condvar::new()),
                child_exit_code: BTreeMap::new(),
            })),
        })
    }

    /// Make a new user process from ELF `data`
    pub fn new_user<'a, Iter>(data: &[u8], args: Iter) -> Box<Thread>
    where
        Iter: Iterator<Item = &'a str>,
    {
        // Parse ELF
        let elf = ElfFile::new(data).expect("failed to read elf");

        // Check ELF type
        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {}
            header::Type::SharedObject => {}
            _ => panic!("ELF is not executable or shared object"),
        }

        // Check interpreter
        if let Ok(loader_path) = elf.get_interpreter() {
            // assuming absolute path
            if let Ok(inode) = crate::fs::ROOT_INODE.lookup_follow(loader_path, FOLLOW_MAX_DEPTH) {
                if let Ok(buf) = inode.read_as_vec() {
                    debug!("using loader {}", &loader_path);
                    // Elf loader should not have INTERP
                    // No infinite loop
                    let mut new_args: Vec<&str> = args.collect();
                    new_args.insert(0, loader_path);
                    return Thread::new_user(buf.as_slice(), new_args.into_iter());
                } else {
                    warn!("loader specified as {} but failed to read", &loader_path);
                }
            } else {
                warn!("loader specified as {} but not found", &loader_path);
            }
        }

        // Make page table
        let mut vm = elf.make_memory_set();

        // User stack
        use crate::consts::{USER_STACK_OFFSET, USER_STACK_SIZE};
        let mut ustack_top = {
            let ustack_buttom = USER_STACK_OFFSET;
            let ustack_top = USER_STACK_OFFSET + USER_STACK_SIZE;
            vm.push(
                ustack_buttom,
                ustack_top,
                MemoryAttr::default().user(),
                ByFrame::new(GlobalFrameAlloc),
                "user_stack",
            );
            ustack_top
        };

        // Make init info
        let init_info = ProcInitInfo {
            args: args.map(|s| String::from(s)).collect(),
            envs: BTreeMap::new(),
            auxv: {
                let mut map = BTreeMap::new();
                if let Some(phdr_vaddr) = elf.get_phdr_vaddr() {
                    map.insert(abi::AT_PHDR, phdr_vaddr as usize);
                }
                map.insert(abi::AT_PHENT, elf.header.pt2.ph_entry_size() as usize);
                map.insert(abi::AT_PHNUM, elf.header.pt2.ph_count() as usize);
                map.insert(abi::AT_PAGESZ, PAGE_SIZE);
                map
            },
        };
        unsafe {
            vm.with(|| ustack_top = init_info.push_at(ustack_top));
        }

        trace!("{:#x?}", vm);

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
                },
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
                },
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
                },
            )),
        );

        let entry_addr = elf.header.pt2.entry_point() as usize;

        Box::new(Thread {
            context: unsafe {
                Context::new_user_thread(entry_addr, ustack_top, kstack.top(), vm.token())
            },
            kstack,
            clear_child_tid: 0,
            proc: Arc::new(Mutex::new(Process {
                vm,
                files,
                cwd: String::from("/"),
                futexes: BTreeMap::default(),
                pid: Pid::uninitialized(),
                parent: None,
                children: Vec::new(),
                threads: Vec::new(),
                child_exit: Arc::new(Condvar::new()),
                child_exit_code: BTreeMap::new(),
            })),
        })
    }

    /// Fork a new process from current one
    pub fn fork(&self, tf: &TrapFrame) -> Box<Thread> {
        // Clone memory set, make a new page table
        let proc = self.proc.lock();
        let vm = proc.vm.clone();
        let files = proc.files.clone();
        let cwd = proc.cwd.clone();
        drop(proc);
        let parent = Some(self.proc.clone());
        debug!("fork: finish clone MemorySet");

        // MMU:   copy data to the new space
        // NoMMU: coping data has been done in `vm.clone()`
        for area in vm.iter() {
            let data = Vec::<u8>::from(unsafe { area.as_slice() });
            unsafe { vm.with(|| area.as_slice_mut().copy_from_slice(data.as_slice())) }
        }

        debug!("fork: temporary copy data!");
        let kstack = KernelStack::new();

        Box::new(Thread {
            context: unsafe { Context::new_fork(tf, kstack.top(), vm.token()) },
            kstack,
            clear_child_tid: 0,
            proc: Arc::new(Mutex::new(Process {
                vm,
                files,
                cwd,
                futexes: BTreeMap::default(),
                pid: Pid::uninitialized(),
                parent,
                children: Vec::new(),
                threads: Vec::new(),
                child_exit: Arc::new(Condvar::new()),
                child_exit_code: BTreeMap::new(),
            })),
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
        let token = self.proc.lock().vm.token();
        Box::new(Thread {
            context: unsafe { Context::new_clone(tf, stack_top, kstack.top(), token, tls) },
            kstack,
            clear_child_tid,
            proc: self.proc.clone(),
        })
    }
}

impl Process {
    pub fn get_free_fd(&self) -> usize {
        (0..).find(|i| !self.files.contains_key(i)).unwrap()
    }
    pub fn get_futex(&mut self, uaddr: usize) -> Arc<Condvar> {
        if !self.futexes.contains_key(&uaddr) {
            self.futexes.insert(uaddr, Arc::new(Condvar::new()));
        }
        self.futexes.get(&uaddr).unwrap().clone()
    }
    pub fn clone_for_exec(&mut self, other: &Self) {
        self.files = other.files.clone();
        self.cwd = other.cwd.clone();
        self.pid = other.pid.clone();
        self.parent = other.parent.clone();
        self.threads = other.threads.clone();
    }
}

trait ToMemoryAttr {
    fn to_attr(&self) -> MemoryAttr;
}

impl ToMemoryAttr for Flags {
    fn to_attr(&self) -> MemoryAttr {
        let mut flags = MemoryAttr::default().user();
        // FIXME: handle readonly
        if self.is_execute() {
            flags = flags.execute();
        }
        flags
    }
}

/// Helper functions to process ELF file
trait ElfExt {
    /// Generate a MemorySet according to the ELF file.
    fn make_memory_set(&self) -> MemorySet;

    /// Get interpreter string if it has.
    fn get_interpreter(&self) -> Result<&str, &str>;

    /// Get virtual address of PHDR section if it has.
    fn get_phdr_vaddr(&self) -> Option<u64>;
}

impl ElfExt for ElfFile<'_> {
    fn make_memory_set(&self) -> MemorySet {
        debug!("creating MemorySet from ELF");
        let mut ms = MemorySet::new();

        for ph in self.program_iter() {
            if ph.get_type() != Ok(Type::Load) {
                continue;
            }
            let virt_addr = ph.virtual_addr() as usize;
            let mem_size = ph.mem_size() as usize;
            let data = match ph.get_data(self).unwrap() {
                SegmentData::Undefined(data) => data,
                _ => unreachable!(),
            };

            // Get target slice
            let target = {
                ms.push(
                    virt_addr,
                    virt_addr + mem_size,
                    ph.flags().to_attr(),
                    ByFrame::new(GlobalFrameAlloc),
                    "",
                );
                unsafe { ::core::slice::from_raw_parts_mut(virt_addr as *mut u8, mem_size) }
            };
            // Copy data
            unsafe {
                ms.with(|| {
                    if data.len() != 0 {
                        target[..data.len()].copy_from_slice(data);
                    }
                    target[data.len()..].iter_mut().for_each(|x| *x = 0);
                });
            }
        }
        ms
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
