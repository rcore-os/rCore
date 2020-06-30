use crate::drivers::SERIAL_DRIVERS;
use core::fmt::{Arguments, Write};

pub fn putfmt(fmt: Arguments) {
    // output to serial
    #[cfg(not(feature = "board_pc"))]
    {
        let mut drivers = SERIAL_DRIVERS.write();
        let serial = drivers.first_mut().unwrap();
        serial.write(format!("{}", fmt).as_bytes());
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
