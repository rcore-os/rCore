use spin::Once;
use sync::{SpinNoIrqLock, Mutex, MutexGuard, SpinNoIrq};
pub use self::context::ContextImpl;
pub use ucore_process::*;
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


// Implement dependencies for std::thread

#[no_mangle]
pub fn processor() -> &'static Processor {
    &PROCESSORS[cpu::id()]
}

#[no_mangle]
pub fn new_kernel_context(entry: extern fn(usize) -> !, arg: usize) -> Box<Context> {
    ContextImpl::new_kernel(entry, arg)
}