use super::driver::serial::*;
use crate::drivers::console::CONSOLE;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe {
        COM1.force_unlock();
    }
    COM1.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    // output to serial
    unsafe {
        COM1.force_unlock();
    }
    COM1.lock().write_fmt(fmt).unwrap();

    // print to graphic
    #[cfg(feature = "consolegraphic")]
    {
        unsafe { CONSOLE.force_unlock() }
        if let Some(console) = CONSOLE.lock().as_mut() {
            console.write_fmt(fmt).unwrap();
        }
    }
}
