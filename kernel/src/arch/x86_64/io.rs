use super::driver::serial::*;
use super::driver::vga::VGA_WRITER;
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
        unsafe {
            VGA_WRITER.force_unlock();
        }
        VGA_WRITER.lock().write_fmt(fmt).unwrap();
    }
}
