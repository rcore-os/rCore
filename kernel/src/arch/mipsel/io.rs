//! Input/output for mipsel.

use super::driver::serial::*;
use crate::drivers::console::CONSOLE;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().getchar()
}

pub fn getchar_option() -> Option<char> {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().getchar_option()
}

pub fn putfmt(fmt: Arguments) {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().write_fmt(fmt).unwrap();

    unsafe { CONSOLE.force_unlock() }
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.write_fmt(fmt).unwrap();
    }
}
