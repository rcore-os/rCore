use super::bbl::sbi;
use core::fmt::{Write, Result, Arguments};

struct SerialPort;

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            sbi::console_putchar(c as usize);
        }
        Ok(())
    }
}

pub fn getchar() -> char {
    sbi::console_getchar() as u8 as char
}

pub fn putfmt(fmt: Arguments) {
    SerialPort.write_fmt(fmt).unwrap();
}
