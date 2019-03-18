use riscv::register::*;
use super::sbi;
use log::*;

/*
* @brief: 
*   get timer cycle for 64 bit cpu
*/ 
#[cfg(target_pointer_width = "64")]
pub fn get_cycle() -> u64 {
    time::read() as u64
}

/*
* @brief: 
*   get timer cycle for 32 bit cpu
*/ 
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

/*
* @brief: 
*   enable supervisor timer interrupt and set next timer interrupt
*/
pub fn init() {
    // Enable supervisor timer interrupt
    #[cfg(feature = "m_mode")]
    unsafe { mie::set_mtimer(); }
    #[cfg(not(feature = "m_mode"))]
    unsafe { sie::set_stimer(); }
    #[cfg(feature = "board_k210")]
    unsafe { assert_eq!(clint_timer_init(), 0); }
    set_next();
    info!("timer: init end");
}

/*
* @brief: 
*   set the next timer interrupt
*/
#[cfg(not(feature = "board_k210"))]
pub fn set_next() {
    // 100Hz @ QEMU
    let timebase = 250000;
    sbi::set_timer(get_cycle() + timebase);
}

#[cfg(feature = "board_k210")]
pub fn set_next() {
    unsafe {
        assert_eq!(clint_timer_start(10, true), 0);
        mstatus::clear_mie();   // mie is set on 'clint_timer_start'
    }
}

#[link(name = "kendryte")]
#[cfg(feature = "board_k210")]
extern "C" {
    fn clint_timer_init() -> i32;
    fn clint_timer_start(interval_ms: u64, single_shot: bool) -> i32;
}
