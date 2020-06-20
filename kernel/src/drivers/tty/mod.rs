use super::Driver;
use super::TTY_DRIVERS;

#[cfg(target_arch = "x86_64")]
pub mod com;

pub trait TtyDriver: Driver {
    // read one byte from tty
    fn read(&self) -> u8;

    // write bytes to tty
    fn write(&self, data: &[u8]);
}
