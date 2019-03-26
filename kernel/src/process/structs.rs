use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, vec::Vec, sync::Weak};
use core::fmt;

use log::*;
use spin::{Mutex, RwLock};
use xmas_elf::{ElfFile, header, program::{Flags, Type}};
use rcore_memory::PAGE_SIZE;
use rcore_thread::Tid;
use core::str;

use crate::arch::interrupt::{Context, TrapFrame};
use crate::memory::{ByFrame, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet};
use crate::fs::{FileHandle, OpenOptions, INodeExt, FOLLOW_MAX_DEPTH};
use crate::sync::Condvar;
use crate::net::{SocketWrapper, SOCKETS};

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


#[derive(Clone)]
pub enum FileLike {
    File(FileHandle),
    Socket(SocketWrapper)
}

impl fmt::Debug for FileLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileLike::File(_) => write!(f, "File"),
            FileLike::Socket(wrapper) => {
                write!(f, "{:?}", wrapper)
            },
        }
    }
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
    pub static ref PROCESSES: RwLock<BTreeMap<usize, Weak<Mutex<Process>>>> = RwLock::new(BTreeMap::new());
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
        PROCESSES.write().insert(proc.pid.get(), Arc::downgrade(&self.proc));
    }
}

impl Thread {
    /// Make a struct for the init thread
    /// TODO: remove this, we only need `Context::null()`
    pub unsafe fn new_init() -> Box<Thread> {
        Box::new(Thread {
            context: Context::null(),
            kstack: KernelStack::new(),
            clear_child_tid: 0,
            proc: Arc::new(Mutex::new(Process {
                vm: MemorySet::new(),
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

    /// Make a new kernel thread starting from `entry` with `arg`
    pub fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Box<Thread> {
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
                child_exit_code: BTreeMap::new()
            })),
        })
    }

    /// Make a new user process from ELF `data`
    pub fn new_user<'a, Iter>(data: &[u8], args: Iter) -> Box<Thread>
        where Iter: Iterator<Item=&'a str>
    {
        // Parse elf
        let elf = ElfFile::new(data).expect("failed to read elf");
        let is32 = match elf.header.pt2 {
            header::HeaderPt2::Header32(_) => true,
            header::HeaderPt2::Header64(_) => false,
        };

        match elf.header.pt2.type_().as_type() {
            header::Type::Executable => {
//                panic!("ELF is not shared object");
            },
            header::Type::SharedObject => {},
            _ => panic!("ELF is not executable or shared object"),
        }

        if let Some(interp) = elf.program_iter().filter(|ph| ph.get_type() == Ok(Type::Interp)).next() {
            let offset = interp.offset() as usize;
            let size = interp.file_size() as usize - 1; // skip last '\0'
            if offset + size < data.len() {
                if let Ok(loader_path) = str::from_utf8(&data[offset..(offset+size)]) {
                    // assuming absolute path
                    if let Ok(inode) = crate::fs::ROOT_INODE.lookup_follow(&loader_path[1..], FOLLOW_MAX_DEPTH) {
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
                } else {
                    warn!("loader specified but not found");
                }
            } else {
                warn!("loader specified but invalid");
            }
        }

        // Make page table
        let (mut vm, entry_addr) = memory_set_from(&elf);

        // User stack
        use crate::consts::{USER_STACK_OFFSET, USER_STACK_SIZE, USER32_STACK_OFFSET};
        let mut ustack_top = {
            let (ustack_buttom, ustack_top) = match is32 {
                true => (USER32_STACK_OFFSET, USER32_STACK_OFFSET + USER_STACK_SIZE),
                false => (USER_STACK_OFFSET, USER_STACK_OFFSET + USER_STACK_SIZE),
            };
            vm.push(ustack_buttom, ustack_top,  MemoryAttr::default().user(), ByFrame::new(GlobalFrameAlloc), "user_stack");
            ustack_top
        };

        let init_info = ProcInitInfo {
            args: args.map(|s| String::from(s)).collect(),
            envs: BTreeMap::new(),
            auxv: {
                let mut map = BTreeMap::new();
                if let Some(phdr) = elf.program_iter()
                    .find(|ph| ph.get_type() == Ok(Type::Phdr)) {
                    // if phdr exists in program header, use it
                    map.insert(abi::AT_PHDR, phdr.virtual_addr() as usize);
                } else if let Some(elf_addr) = elf.program_iter().find(|ph| ph.get_type() == Ok(Type::Load) && ph.offset() == 0) {
                    // otherwise, check if elf is loaded from the beginning, then phdr can be inferred.
                    map.insert(abi::AT_PHDR, elf_addr.virtual_addr() as usize + elf.header.pt2.ph_offset() as usize);
                } else {
                    warn!("new_user: no phdr found, tls might not work");
                }
                map.insert(abi::AT_PHENT, elf.header.pt2.ph_entry_size() as usize);
                map.insert(abi::AT_PHNUM, elf.header.pt2.ph_count() as usize);
                map.insert(abi::AT_PAGESZ, PAGE_SIZE);
                map
            },
        };
        unsafe {
            vm.with(|| { ustack_top = init_info.push_at(ustack_top) });
        }

        trace!("{:#x?}", vm);

        let kstack = KernelStack::new();

        let mut files = BTreeMap::new();
        files.insert(0, FileLike::File(FileHandle::new(crate::fs::STDIN.clone(), OpenOptions { read: true, write: false, append: false })));
        files.insert(1, FileLike::File(FileHandle::new(crate::fs::STDOUT.clone(), OpenOptions { read: false, write: true, append: false })));
        files.insert(2, FileLike::File(FileHandle::new(crate::fs::STDOUT.clone(), OpenOptions { read: false, write: true, append: false })));

        Box::new(Thread {
            context: unsafe {
                Context::new_user_thread(
                    entry_addr, ustack_top, kstack.top(), is32, vm.token())
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
                child_exit_code: BTreeMap::new()
            })),
        })
    }

    /// Fork a new process from current one
    pub fn fork(&self, tf: &TrapFrame) -> Box<Thread> {
        // Clone memory set, make a new page table
        let vm = self.proc.lock().vm.clone();
        let files = self.proc.lock().files.clone();
        let cwd = self.proc.lock().cwd.clone();
        let parent = Some(self.proc.clone());
        debug!("fork: finish clone MemorySet");

        // MMU:   copy data to the new space
        // NoMMU: coping data has been done in `vm.clone()`
        for area in vm.iter() {
            let data = Vec::<u8>::from(unsafe { area.as_slice() });
            unsafe { vm.with(|| {
                area.as_slice_mut().copy_from_slice(data.as_slice())
            }) }
        }

        debug!("fork: temporary copy data!");
        let kstack = KernelStack::new();

        let mut sockets = SOCKETS.lock();
        for (_fd, file) in files.iter() {
            if let FileLike::Socket(wrapper) = file {
                sockets.retain(wrapper.handle);
            }
        }


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
                child_exit_code: BTreeMap::new()
            })),
        })
    }

    /// Create a new thread in the same process.
    pub fn clone(&self, tf: &TrapFrame, stack_top: usize, tls: usize, clear_child_tid: usize) -> Box<Thread> {
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


/// Generate a MemorySet according to the ELF file.
/// Also return the real entry point address.
fn memory_set_from(elf: &ElfFile<'_>) -> (MemorySet, usize) {
    debug!("creating MemorySet from ELF");
    let mut ms = MemorySet::new();
    let entry = elf.header.pt2.entry_point() as usize;

    // [NoMMU] Get total memory size and alloc space
    let va_begin = elf.program_iter()
        .filter(|ph| ph.get_type() == Ok(Type::Load))
        .map(|ph| ph.virtual_addr()).min().unwrap() as usize;
    let va_end = elf.program_iter()
        .filter(|ph| ph.get_type() == Ok(Type::Load))
        .map(|ph| ph.virtual_addr() + ph.mem_size()).max().unwrap() as usize;
    let va_size = va_end - va_begin;

    for ph in elf.program_iter() {
        if ph.get_type() != Ok(Type::Load) {
            continue;
        }
        let virt_addr = ph.virtual_addr() as usize;
        let offset = ph.offset() as usize;
        let file_size = ph.file_size() as usize;
        let mem_size = ph.mem_size() as usize;

        // Get target slice
        let target = {
            ms.push(virt_addr, virt_addr + mem_size, ph.flags().to_attr(), ByFrame::new(GlobalFrameAlloc), "");
            unsafe { ::core::slice::from_raw_parts_mut(virt_addr as *mut u8, mem_size) }
        };
        // Copy data
        unsafe {
            ms.with(|| {
                if file_size != 0 {
                    target[..file_size].copy_from_slice(&elf.input[offset..offset + file_size]);
                }
                target[file_size..].iter_mut().for_each(|x| *x = 0);
            });
        }
    }
    (ms, entry)
}

trait ToMemoryAttr {
    fn to_attr(&self) -> MemoryAttr;
}

impl ToMemoryAttr for Flags {
    fn to_attr(&self) -> MemoryAttr {
        let mut flags = MemoryAttr::default().user();
        // FIXME: handle readonly
        if self.is_execute() { flags = flags.execute(); }
        flags
    }
}
