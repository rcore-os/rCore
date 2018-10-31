use super::bcm2837::timer;
use super::bcm2837::interrupt::{Controller, Interrupt};

pub fn init() {
    Controller::new().enable(Interrupt::Timer1);
    set_next();
}

pub fn get_cycle() -> u64 {
    timer::current_time()
}

pub fn set_next() {
    // 1000 ms
    timer::tick_in(timer::SystemTimer::Timer1, 1000 * 1000);
}
