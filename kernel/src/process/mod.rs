use alloc::String;
pub use self::context::*;
pub use self::processor::*;
use spin::Once;
use sync::{SpinNoIrqLock, Mutex, MutexGuard, SpinNoIrq};

mod context;
mod processor;
mod scheduler;

pub fn init() {
    PROCESSOR.call_once(||
        SpinNoIrqLock::new(Processor::new(unsafe { Context::new_init() }))
    );
}

pub static PROCESSOR: Once<SpinNoIrqLock<Processor>> = Once::new();

pub fn processor() -> MutexGuard<'static, Processor, SpinNoIrq> {
    PROCESSOR.try().unwrap().lock()
}