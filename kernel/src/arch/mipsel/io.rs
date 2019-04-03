//! Input/output for mipsel.

use super::driver::console::CONSOLE;
use super::driver::serial::*;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().getchar()
}

pub fn getchar_option() -> Option<Char> {
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

pub fn putchar(c: u8) {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().putchar(c);
}
