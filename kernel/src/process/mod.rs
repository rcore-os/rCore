pub use self::structs::*;
use crate::arch::cpu;
use crate::consts::{MAX_CPU_NUM, MAX_PROCESS_NUM};
use crate::sync::{MutexGuard, SpinNoIrq};
use alloc::{boxed::Box, sync::Arc};
use log::*;
pub use rcore_thread::*;

mod abi;
pub mod structs;

pub fn init() {
    // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
    let scheduler = scheduler::RRScheduler::new(5);
    let manager = Arc::new(ThreadPool::new(scheduler, MAX_PROCESS_NUM));

    unsafe {
        for cpu_id in 0..MAX_CPU_NUM {
            PROCESSORS[cpu_id].init(cpu_id, Thread::new_init(), manager.clone());
        }
    }

    crate::shell::add_user_shell();

    info!("process: init end");
}

static PROCESSORS: [Processor; MAX_CPU_NUM] = [
    // TODO: More elegant ?
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
    //    Processor::new(),    Processor::new(),    Processor::new(),    Processor::new(),
];

/// Get current process
pub fn process() -> MutexGuard<'static, Process, SpinNoIrq> {
    current_thread().proc.lock()
}

/// Get current process, ignoring its lock
/// Only use this when necessary
pub unsafe fn process_unsafe() -> MutexGuard<'static, Process, SpinNoIrq> {
    let thread = current_thread();
    thread.proc.force_unlock();
    thread.proc.lock()
}

/// Get current thread
///
/// FIXME: It's obviously unsafe to get &mut !
pub fn current_thread() -> &'static mut Thread {
    use core::mem::transmute;
    let (process, _): (&mut Thread, *const ()) = unsafe { transmute(processor().context()) };
    process
}

// Implement dependencies for std::thread

#[no_mangle]
pub fn processor() -> &'static Processor {
    &PROCESSORS[cpu::id()]
}

#[no_mangle]
pub fn new_kernel_context(entry: extern "C" fn(usize) -> !, arg: usize) -> Box<Context> {
    Thread::new_kernel(entry, arg)
}
