//! Serial driver for aarch64.

use core::fmt::{Arguments, Write};
use super::board::serial::*;

pub fn getchar() -> char {
    unsafe { SERIAL_PORT.force_unlock(); }
    SERIAL_PORT.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    unsafe { SERIAL_PORT.force_unlock(); }
    SERIAL_PORT.lock().write_fmt(fmt).unwrap()
}
