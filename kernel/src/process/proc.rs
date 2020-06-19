use super::{
    abi::{self, ProcInitInfo},
    Tid,
};
use crate::arch::interrupt::TrapFrame;
use crate::arch::paging::*;
use crate::fs::{FileHandle, FileLike, OpenOptions, FOLLOW_MAX_DEPTH};
use crate::ipc::SemProc;
use crate::memory::{
    phys_to_virt, ByFrame, Delay, File, GlobalFrameAlloc, KernelStack, MemoryAttr, MemorySet, Read,
};
use crate::sync::{Condvar, Event, EventBus, SpinLock, SpinNoIrqLock as Mutex};
use crate::{
    signal::{Siginfo, Signal, SignalAction, SignalStack, Sigset},
    syscall::handle_syscall,
};
use alloc::{
    boxed::Box, collections::BTreeMap, collections::VecDeque, string::String, sync::Arc,
    sync::Weak, vec::Vec,
};
use apic::{LocalApic, XApic, LAPIC_ADDR};
use bitflags::_core::cell::Ref;
use core::fmt;
use core::str;
use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};
use log::*;
use pc_keyboard::KeyCode::BackTick;
use rcore_fs::vfs::INode;
use rcore_memory::{Page, PAGE_SIZE};
use spin::RwLock;
use trapframe::UserContext;
use x86_64::{
    registers::control::{Cr2, Cr3, Cr3Flags},
    structures::paging::PhysFrame,
    PhysAddr, VirtAddr,
};
use xmas_elf::{
    header,
    program::{Flags, SegmentData, Type},
    ElfFile,
};

/// Pid type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pid(usize);

impl Pid {
    pub const INIT: usize = 1;

    pub fn new() -> Self {
        Pid(0)
    }

    pub fn get(&self) -> usize {
        self.0
    }

    /// Return whether this pid represents the init process
    pub fn is_init(&self) -> bool {
        self.0 == Self::INIT
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// process group id type
pub type Pgid = i32;

pub struct Process {
    /// Virtual memory
    pub vm: Arc<Mutex<MemorySet>>,

    /// Opened files
    pub files: BTreeMap<usize, FileLike>,

    /// Current working dirctory
    pub cwd: String,

    /// Executable path
    pub exec_path: String,

    /// Futex
    pub futexes: BTreeMap<usize, Arc<Condvar>>,

    /// Semaphore
    pub semaphores: SemProc,

    /// Pid i.e. tgid, usually the tid of first thread
    pub pid: Pid,

    //// Process group id
    pub pgid: Pgid,

    /// Parent process
    /// Avoid deadlock, put pid out
    pub parent: (Pid, Weak<Mutex<Process>>),

    /// Children process
    pub children: Vec<(Pid, Weak<Mutex<Process>>)>,

    /// Threads
    /// threads in the same process
    pub threads: Vec<Tid>,

    /// Events like exiting
    pub eventbus: Arc<Mutex<EventBus>>,

    /// Exit code
    pub exit_code: usize,

    // delivered signals, tid specified thread, -1 stands for any thread
    // TODO: implement with doubly linked list, but how to do it in rust safely? [doggy]
    pub sig_queue: VecDeque<(Siginfo, isize)>,
    pub pending_sigset: Sigset,

    pub dispositions: [SignalAction; Signal::RTMAX + 1],
    pub sigaltstack: SignalStack,
}

lazy_static! {
    /// Records the mapping between pid and Process struct.
    pub static ref PROCESSES: RwLock<BTreeMap<usize, Arc<Mutex<Process>>>> =
        RwLock::new(BTreeMap::new());
}

/// Return the process which thread tid is in
pub fn process_of(tid: usize) -> Option<Arc<Mutex<Process>>> {
    PROCESSES
        .read()
        .iter()
        .map(|(_, proc)| proc.clone())
        .find(|proc| proc.lock().threads.contains(&tid))
}

/// Get process by pid
pub fn process(pid: usize) -> Option<Arc<Mutex<Process>>> {
    PROCESSES.read().get(&pid).cloned()
}

/// Get process group by pgid
pub fn process_group(pgid: Pgid) -> Vec<Arc<Mutex<Process>>> {
    PROCESSES
        .read()
        .iter()
        .map(|(_, proc)| proc.clone())
        .filter(|proc| proc.lock().pgid == pgid)
        .collect::<Vec<_>>()
}

impl Process {
    /// Assign a pid and put itself to global process table.
    pub fn add_to_table(mut self) -> Arc<Mutex<Self>> {
        let mut process_table = PROCESSES.write();

        // assign pid, do not start from 0
        let pid = (Pid::INIT..)
            .find(|i| process_table.get(i).is_none())
            .unwrap();
        self.pid = Pid(pid);

        // put to process table
        let self_ref = Arc::new(Mutex::new(self));
        process_table.insert(pid, self_ref.clone());

        self_ref
    }

    /// Get lowest free fd
    fn get_free_fd(&self) -> usize {
        (0..).find(|i| !self.files.contains_key(i)).unwrap()
    }

    /// get the lowest available fd great than or equal to arg
    pub fn get_free_fd_from(&self, arg: usize) -> usize {
        (arg..).find(|i| !self.files.contains_key(i)).unwrap()
    }

    /// Add a file to the process, return its fd.
    pub fn add_file(&mut self, file_like: FileLike) -> usize {
        let fd = self.get_free_fd();
        self.files.insert(fd, file_like);
        fd
    }

    /// Get futex by addr
    pub fn get_futex(&mut self, uaddr: usize) -> Arc<Condvar> {
        if !self.futexes.contains_key(&uaddr) {
            self.futexes.insert(uaddr, Arc::new(Condvar::new()));
        }
        self.futexes.get(&uaddr).unwrap().clone()
    }

    /// Exit the process.
    /// Kill all threads and notify parent with the exit code.
    pub fn exit(&mut self, exit_code: usize) {
        // avoid some strange dead lock
        // self.files.clear(); this does not work sometime, for unknown reason
        // manually drop
        let fds = self.files.iter().map(|(fd, _)| *fd).collect::<Vec<_>>();
        for fd in fds.iter() {
            let file = self.files.remove(fd).unwrap();
            drop(file);
        }

        // notify parent and fill exit code
        self.eventbus.lock().set(Event::PROCESS_QUIT);
        if let Some(parent) = self.parent.1.upgrade() {
            parent.lock().eventbus.lock().set(Event::CHILD_PROCESS_QUIT);
        }
        self.exit_code = exit_code;

        // quit all threads
        // this must be after setting the value of subprocess, or the threads will be treated exit before actually exits
        for tid in self.threads.iter() {
            //thread_manager().exit(*tid, 1);
        }
        self.threads.clear();

        info!("process {} exit with {}", self.pid.get(), exit_code);
    }

    pub fn exited(&self) -> bool {
        self.threads.is_empty()
    }
}