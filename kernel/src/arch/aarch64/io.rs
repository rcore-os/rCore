//! Input/output for aarch64.

use super::driver::serial::*;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    unsafe { SERIAL_PORT.force_unlock() }
    SERIAL_PORT.lock().write_fmt(fmt).unwrap();

    // print to graphic
    #[cfg(feature = "consolegraphic")]
    {
        use crate::drivers::console::CONSOLE;
        unsafe { CONSOLE.force_unlock() }
        if let Some(console) = CONSOLE.lock().as_mut() {
            console.write_fmt(fmt).unwrap();
        }
    }
}
