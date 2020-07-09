//! Input/output for mipsel.

use crate::drivers::{console::CONSOLE, SERIAL_DRIVERS};
use core::fmt::{Arguments, Write};

pub fn putfmt(fmt: Arguments) {
    // output to serial
    let mut drivers = SERIAL_DRIVERS.write();
    if let Some(serial) = drivers.first_mut() {
        serial.write(format!("{}", fmt).as_bytes());
    }

    unsafe { CONSOLE.force_unlock() }
    if let Some(console) = CONSOLE.lock().as_mut() {
        console.write_fmt(fmt).unwrap();
    }
}
