use super::riscv::register::*;
use super::bbl::sbi;

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

pub fn init() {
    // Enable supervisor timer interrupt
    unsafe { sie::set_stimer(); }

    set_next();
    info!("timer: init end");
}

pub fn set_next() {
    // 100Hz @ QEMU
    let timebase = 250000;
    set_timer(get_cycle() + timebase);
}

fn set_timer(t: u64) {
    #[cfg(feature = "no_bbl")]
    unsafe {
        asm!("csrw 0x321, $0; csrw 0x322, $1"
        : : "r"(t as u32), "r"((t >> 32) as u32) : : "volatile");
    }
    #[cfg(not(feature = "no_bbl"))]
    sbi::set_timer(t);
}