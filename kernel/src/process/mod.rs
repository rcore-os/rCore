pub use self::structs::*;
use crate::arch::cpu;
use crate::consts::{MAX_CPU_NUM, MAX_PROCESS_NUM};
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

/// Get current thread
///
/// `Thread` is a thread-local object.
/// It is safe to call this once, and pass `&mut Thread` as a function argument.
pub unsafe fn current_thread() -> &'static mut Thread {
    // trick: force downcast from trait object
    let (process, _): (&mut Thread, *const ()) = core::mem::transmute(processor().context());
    process
}

// Implement dependencies for std::thread

#[no_mangle]
pub fn processor() -> &'static Processor {
    &PROCESSORS[cpu::id()]
}

#[no_mangle]
pub fn new_kernel_context(entry: extern "C" fn(usize) -> !, arg: usize) -> Box<dyn Context> {
    Thread::new_kernel(entry, arg)
}
