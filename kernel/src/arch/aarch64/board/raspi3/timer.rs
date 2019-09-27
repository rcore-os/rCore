use crate::consts::USEC_PER_TICK;
use bcm2837::timer::{BasicTimer, Timer};
use log::*;

/// Initialization timer.
pub fn init() {
    Timer::new().init();
    set_next();
    info!("timer: init end");
}

/// Set next timer interrupt to 10 ms from now.
pub fn set_next() {
    Timer::new().tick_in(USEC_PER_TICK);
}

/// Is interrupt pending
pub fn is_pending() -> bool {
    Timer::new().is_pending()
}
