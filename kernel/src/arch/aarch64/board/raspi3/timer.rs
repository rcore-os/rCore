use bcm2837::timer;
use log::*;

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
