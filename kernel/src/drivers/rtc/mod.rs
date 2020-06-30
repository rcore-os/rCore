use super::Driver;
use super::RTC_DRIVERS;

#[cfg(target_arch = "x86_64")]
pub mod rtc_cmos;
#[cfg(riscv)]
pub mod rtc_goldfish;

pub trait RtcDriver: Driver {
    // read seconds since epoch
    fn read_epoch(&self) -> u64;
}

/// Try to read epoch from one rtc driver
pub fn read_epoch() -> u64 {
    let drivers = RTC_DRIVERS.read();
    if let Some(driver) = drivers.first() {
        driver.read_epoch()
    } else {
        0
    }
}
