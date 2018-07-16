use super::bbl::sbi;
use core::fmt::{Write, Result, Arguments};

struct SerialPort;

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            if c == 127 {
                sbi::console_putchar(8);
                sbi::console_putchar(' ' as usize);
                sbi::console_putchar(8);
            } else {
                sbi::console_putchar(c as usize);
            }
        }
        Ok(())
    }
}

pub fn getchar() -> char {
    match sbi::console_getchar() as u8 {
        255 => '\0',   // null
        c => c as char,
    }
}

pub fn putfmt(fmt: Arguments) {
    SerialPort.write_fmt(fmt).unwrap();
}
