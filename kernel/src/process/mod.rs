pub use self::structs::*;
pub use rcore_thread::*;
use crate::consts::{MAX_CPU_NUM, MAX_PROCESS_NUM};
use crate::arch::cpu;
use alloc::{boxed::Box, sync::Arc};
use spin::MutexGuard;
use log::*;

pub mod structs;
mod abi;

pub fn init() {
    // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
    let scheduler = Box::new(scheduler::RRScheduler::new(5));
    let manager = Arc::new(ThreadPool::new(scheduler, MAX_PROCESS_NUM));

    unsafe {
        for cpu_id in 0..MAX_CPU_NUM {
            PROCESSORS[cpu_id].init(cpu_id, Thread::new_init(), manager.clone());
        }
    }

    crate::shell::run_user_shell();

    info!("process init end");
}

static PROCESSORS: [Processor; MAX_CPU_NUM] = [Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new()];

/// Get current process
pub fn process() -> MutexGuard<'static, Process> {
    current_thread().proc.lock()
}

/// Get current thread
///
/// FIXME: It's obviously unsafe to get &mut !
pub fn current_thread() -> &'static mut Thread {
    use core::mem::transmute;
    let (process, _): (&mut Thread, *const ()) = unsafe {
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
    Thread::new_kernel(entry, arg)
}
