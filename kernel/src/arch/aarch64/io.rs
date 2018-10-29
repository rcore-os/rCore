//! Serial driver for aarch64.

use core::fmt::{Arguments, Write};
use super::board::serial::{SerialRead, SERIAL_PORT};

pub fn getchar() -> char {
    // FIXME
    unsafe {
        SERIAL_PORT.receive() as char
    }
}

pub fn putfmt(fmt: Arguments) {
    // FIXME
    unsafe {
        SERIAL_PORT.write_fmt(fmt).unwrap()
    }
}
