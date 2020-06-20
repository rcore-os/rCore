use super::Driver;
use super::SERIAL_DRIVERS;
use alloc::sync::Arc;
use core::fmt::{Result, Write};

#[cfg(target_arch = "x86_64")]
pub mod com;
#[cfg(target_arch = "x86_64")]
pub mod keyboard;

pub trait SerialDriver: Driver {
    // read one byte from tty
    fn read(&self) -> u8;

    // write bytes to tty
    fn write(&self, data: &[u8]);
}
