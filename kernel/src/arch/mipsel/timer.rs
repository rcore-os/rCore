use core::time::Duration;
use log::*;
use mips::registers::cp0;

static mut TICK: u64 = 0;
const timebase: u32 = 250000;

/// Enable timer interrupt
pub fn init() {
    // Enable supervisor timer interrupt
    cp0::status::enable_hard_int5(); // IP(7), timer interrupt
    cp0::count::write_u32(0);
    set_next();
    info!("timer: init end");
}

/// Set the next timer interrupt
pub fn set_next() {
    // 100Hz @ QEMU
    cp0::count::write_u32(0);
    cp0::compare::write_u32(timebase);
    unsafe {
        TICK += 1;
    }
}

pub fn timer_now() -> Duration {
    let mut curr_time = unsafe { TICK * timebase as u64 };
    curr_time += cp0::count::read_u32() as u64;
    Duration::from_nanos(curr_time * 100)
}
