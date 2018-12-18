//! Serial driver for aarch64.

use super::driver::serial::*;
use super::driver::console::CONSOLE;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().write_fmt(fmt).unwrap();

    unsafe { CONSOLE.force_unlock() }
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.write_fmt(fmt).unwrap();
    }
}
