//! Serial driver for aarch64.

use core::fmt::{Arguments, Write};
use super::board::serial::{SerialRead, SERIAL_PORT};

pub fn getchar() -> char {
    SERIAL_PORT.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    SERIAL_PORT.lock().write_fmt(fmt).unwrap()
}
