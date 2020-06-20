use crate::drivers::SERIAL_DRIVERS;
use core::fmt::{Arguments, Write};

pub fn putfmt(fmt: Arguments) {
    // output to serial
    let mut drivers = SERIAL_DRIVERS.write();
    if let Some(serial) = drivers.first_mut() {
        serial.write(format!("{}", fmt).as_bytes());
    }
    // might miss some early messages
}
