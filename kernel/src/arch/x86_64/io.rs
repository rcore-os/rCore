use super::driver::console::CONSOLE;
use super::driver::serial::*;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe {
        COM1.force_unlock();
    }
    COM1.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    #[cfg(feature = "nographic")]
    {
        unsafe {
            COM1.force_unlock();
        }
        COM1.lock().write_fmt(fmt).unwrap();
    }
    #[cfg(not(feature = "nographic"))]
    {
        use super::driver::vga::VGA_WRITER;
        unsafe {
            COM1.force_unlock();
        }
        COM1.lock().write_fmt(fmt).unwrap();
        unsafe { CONSOLE.force_unlock() }
        if let Some(console) = CONSOLE.lock().as_mut() {
            console.write_fmt(fmt).unwrap();
        }
    }
}
