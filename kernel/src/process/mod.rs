pub use self::context::Process;
pub use rcore_process::*;
use crate::consts::{MAX_CPU_NUM, MAX_PROCESS_NUM};
use crate::arch::cpu;
use alloc::{boxed::Box, sync::Arc};
use log::*;

pub mod context;

pub fn init() {
    // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
    let scheduler = Box::new(scheduler::RRScheduler::new(5));
    let manager = Arc::new(ProcessManager::new(scheduler, MAX_PROCESS_NUM));

    unsafe {
        for cpu_id in 0..MAX_CPU_NUM {
            PROCESSORS[cpu_id].init(cpu_id, Process::new_init(), manager.clone());
        }
    }

    // Add idle threads
    extern fn idle(_arg: usize) -> ! {
        loop { cpu::halt(); }
    }
    use core::str::FromStr;
    let cores = usize::from_str(env!("SMP")).unwrap();
    for i in 0..cores {
        manager.add(Process::new_kernel(idle, i), 0);
    }
    crate::shell::run_user_shell();

    info!("process init end");
}

static PROCESSORS: [Processor; MAX_CPU_NUM] = [Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new(), Processor::new()];

/// Get current thread struct
///
/// FIXME: It's obviously unsafe to get &mut !
pub fn process() -> &'static mut Process {
    use core::mem::transmute;
    let (process, _): (&mut Process, *const ()) = unsafe {
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
    Process::new_kernel(entry, arg)
}
