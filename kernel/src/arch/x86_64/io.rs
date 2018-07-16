use super::driver::serial::*;
use core::fmt::{Arguments, Write};

pub fn getchar() -> char {
    unsafe { COM1.force_unlock(); }
    COM1.lock().receive() as char
}

pub fn putfmt(fmt: Arguments) {
    unsafe { COM1.force_unlock(); }
    COM1.lock().write_fmt(fmt).unwrap()
}