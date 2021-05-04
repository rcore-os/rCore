use super::sbi;
use core::time::Duration;
use log::*;
use riscv::register::*;

#[cfg(target_arch = "riscv64")]
pub fn get_cycle() -> u64 {
    time::read() as u64
}

#[cfg(target_arch = "riscv32")]
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
    let timebase = 100000;
    sbi::sbi_set_timer(get_cycle() + timebase);
}

pub fn timer_now() -> Duration {
    let time = get_cycle();
    Duration::from_nanos(time * 100)
}
