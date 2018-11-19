use super::bcm2837::timer;
use super::bcm2837::interrupt::{Controller, Interrupt};

pub fn init() {
    timer::init();
    set_next();
    info!("timer: init end");
}

pub fn get_cycle() -> u64 {
    timer::current_time()
}

pub fn set_next() {
    // 10 ms
    timer::tick_in(10 * 1000);
}
