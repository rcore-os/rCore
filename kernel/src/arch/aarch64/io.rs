//! Input/output for aarch64.

use crate::drivers::SERIAL_DRIVERS;
use core::fmt::{Arguments, Write};

pub fn putfmt(fmt: Arguments) {
    {
        let mut drivers = SERIAL_DRIVERS.write();
        if let Some(serial) = drivers.first_mut() {
            serial.write(format!("{}", fmt).as_bytes());
        }
    }

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
