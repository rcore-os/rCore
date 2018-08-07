use core::fmt::{Write, Result, Arguments};
use core::ptr::{read_volatile, write_volatile};
use super::bbl::sbi;

struct SerialPort;

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            if c == 127 {
                putchar(8);
                putchar(b' ');
                putchar(8);
            } else {
                putchar(c);
            }
        }
        Ok(())
    }
}

fn putchar(c: u8) {
    #[cfg(feature = "no_bbl")]
    unsafe {
        while read_volatile(STATUS) & CAN_WRITE == 0 {}
        write_volatile(DATA, c as u8);
    }
    #[cfg(not(feature = "no_bbl"))]
    sbi::console_putchar(c as usize);
}

pub fn getchar() -> char {
    #[cfg(feature = "no_bbl")]
    let c = unsafe {
        while read_volatile(STATUS) & CAN_READ == 0 {}
        read_volatile(DATA)
    };
    #[cfg(not(feature = "no_bbl"))]
    let c = sbi::console_getchar() as u8;

    match c {
        255 => '\0',   // null
        c => c as char,
    }
}

pub fn putfmt(fmt: Arguments) {
    SerialPort.write_fmt(fmt).unwrap();
}

const DATA: *mut u8 = 0x10000000 as *mut u8;
const STATUS: *const u8 = 0x10000005 as *const u8;
const CAN_READ: u8 = 1 << 0;
const CAN_WRITE: u8 = 1 << 5;
