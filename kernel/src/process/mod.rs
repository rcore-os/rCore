use spin::Once;
use sync::{SpinNoIrqLock, Mutex, MutexGuard, SpinNoIrq};
pub use self::context::ContextImpl;
pub use ucore_process::*;
pub use ucore_process::thread::*;
use alloc::boxed::Box;
use consts::MAX_CPU_NUM;
use arch::cpu;
use alloc::sync::Arc;
use alloc::vec::Vec;

mod context;

pub fn init() {
    // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
    let scheduler = Box::new(scheduler::RRScheduler::new(5));
    let manager = Arc::new(ProcessManager::new(scheduler));

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

pub fn processor() -> &'static Processor {
    &PROCESSORS[cpu::id()]
}

#[allow(non_camel_case_types)]
pub type thread = ThreadMod<ThreadSupportImpl>;

pub mod thread_ {
    pub type Thread = super::Thread<super::ThreadSupportImpl>;
}

pub struct ThreadSupportImpl;

impl ThreadSupport for ThreadSupportImpl {
    fn processor() -> &'static Processor {
        processor()
    }
    fn new_kernel(entry: extern fn(usize) -> !, arg: usize) -> Box<Context> {
        ContextImpl::new_kernel(entry, arg)
    }
}