use crate::drivers::SerialDriver;
use crate::drivers::SERIAL_DRIVERS;
use alloc::sync::Arc;
use core::fmt::{Arguments, Write};
pub struct HeaplessWrite<T: AsRef<dyn SerialDriver>>(T);
impl<T: AsRef<dyn SerialDriver>> core::fmt::Write for HeaplessWrite<T> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.0.as_ref().write(s.as_bytes());
        Ok(())
    }
}

pub struct HeaplessSBIWrite;
impl core::fmt::Write for HeaplessSBIWrite {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for ch in s.as_bytes() {
            super::sbi::console_putchar(*ch as usize);
        }
        Ok(())
    }
}
pub fn putfmt(fmt: Arguments) {
    // output to serial
    let mut drivers = SERIAL_DRIVERS.write();
    if let Some(serial) = drivers.first_mut() {
        HeaplessWrite(&serial).write_fmt(fmt).unwrap();
    } else {
        // might miss some early messages.
        // no no no i don't accept it.
        // note that we can't use heap here.
        HeaplessSBIWrite.write_fmt(fmt).unwrap();
    }
}
