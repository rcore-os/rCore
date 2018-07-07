use super::bbl::sbi;
use core::fmt;

pub struct SerialPort;

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            sbi::console_putchar(c as usize);
        }
        Ok(())
    }
}