use super::sbi;
use log::*;
use riscv::register::*;

#[cfg(target_pointer_width = "64")]
pub fn get_cycle() -> u64 {
    time::read() as u64
}

#[cfg(target_pointer_width = "32")]
pub fn get_cycle() -> u64 {
    loop {
        let hi = timeh::read();
        let lo = time::read();
        let tmp = timeh::read();
        if hi == tmp {
            return ((hi as u64) << 32) | (lo as u64);
        }
    }
}

pub fn read_epoch() -> u64 {
    // TODO: support RTC
    0
}

/// Enable timer interrupt
pub fn init() {
    // Enable supervisor timer interrupt
    unsafe {
        sie::set_stimer();
    }
    set_next();
    info!("timer: init end");
}

/// Set the next timer interrupt
pub fn set_next() {
    // 100Hz @ QEMU
    let timebase = 250000;
    sbi::set_timer(get_cycle() + timebase);
}
