use spin::Once;
use sync::{SpinNoIrqLock, Mutex, MutexGuard, SpinNoIrq};
pub use self::context::Context;
pub use ucore_process::processor::*;
pub use ucore_process::scheduler::*;

mod context;

type Processor = Processor_<Context, StrideScheduler>;

pub fn init() {
    PROCESSOR.call_once(||
        SpinNoIrqLock::new({
            let mut processor = Processor::new(
                unsafe { Context::new_init() },
                // NOTE: max_time_slice <= 5 to ensure 'priority' test pass
                StrideScheduler::new(5),
            );
            extern fn idle(arg: usize) -> ! {
                loop {}
            }
            processor.add(Context::new_kernel(idle, 0));
            processor
        })
    );
}

pub static PROCESSOR: Once<SpinNoIrqLock<Processor>> = Once::new();

pub fn processor() -> MutexGuard<'static, Processor, SpinNoIrq> {
    PROCESSOR.try().unwrap().lock()
}