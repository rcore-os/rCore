use spin::Mutex;
pub use self::context::ContextImpl;
pub use ucore_process::*;
use consts::{MAX_CPU_NUM, MAX_PROCESS_NUM};
use arch::cpu;
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use sync::Condvar;
use core::sync::atomic::*;

pub mod context;

pub fn init() {
    // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
    let scheduler = Box::new(scheduler::RRScheduler::new(5));
    let manager = Arc::new(ProcessManager::new(scheduler, MAX_PROCESS_NUM, Process::proc_exit));

    extern fn idle(_arg: usize) -> ! {
        loop { cpu::halt(); }
    }
    for i in 0..4 {
        manager.add(ContextImpl::new_kernel(idle, i));
    }

    unsafe {
        for cpu_id in 0..MAX_CPU_NUM {
            PROCESSORS[cpu_id].init(cpu_id, ContextImpl::new_init(), manager.clone());
        }
    }
    info!("process init end");
}

static PROCESSORS: [Processor; MAX_CPU_NUM] = [Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new()];

/// Ugly solution for sys_wait(0) (wait for any child)
#[derive(Default)]
pub struct Process {
    parent: AtomicUsize,
    children: Mutex<Vec<usize>>,
    subproc_exit: Condvar,  // Trigger parent's when exit
}

impl Process {
    pub fn new_fork(pid: usize, parent: usize) {
        PROCESS[pid].parent.store(parent, Ordering::Relaxed);
        PROCESS[pid].subproc_exit._clear();
        PROCESS[parent].children.lock().push(pid);
    }
    pub fn proc_exit(pid: usize) {
        let parent = PROCESS[pid].parent.load(Ordering::Relaxed);
        PROCESS[parent].subproc_exit.notify_all();
    }
    pub fn wait_child() {
        Self::current().subproc_exit._wait();
    }
    pub fn get_children() -> Vec<usize> {
        Self::current().children.lock().clone()
    }
    pub fn do_wait(pid: usize) {
        Self::current().children.lock().retain(|&p| p != pid);
    }
    fn current() -> &'static Self {
        &PROCESS[thread::current().id()]
    }
}

lazy_static! {
    pub static ref PROCESS: Vec<Process> = {
        let mut vec = Vec::new();
        vec.resize_default(MAX_PROCESS_NUM);
        vec
    };
}

/// Get current thread struct
pub fn process() -> &'static mut ContextImpl {
    use core::mem::transmute;
    let (process, _): (&mut ContextImpl, *const ()) = unsafe {
        transmute(processor().context())
    };
    process
}


// Implement dependencies for std::thread

#[no_mangle]
pub fn processor() -> &'static Processor {
    &PROCESSORS[cpu::id()]
}

#[no_mangle]
pub fn new_kernel_context(entry: extern fn(usize) -> !, arg: usize) -> Box<Context> {
    ContextImpl::new_kernel(entry, arg)
}