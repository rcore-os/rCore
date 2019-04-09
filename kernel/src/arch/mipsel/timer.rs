use log::*;
use mips::registers::cp0;

pub fn read_epoch() -> u64 {
    // TODO: support RTC
    0
}

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
    let timebase = 250000;
    cp0::count::write_u32(0);
    cp0::compare::write_u32(timebase);
}
