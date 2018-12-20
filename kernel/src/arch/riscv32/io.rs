use core::fmt::{Write, Result, Arguments};
use bbl::sbi;

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
    if cfg!(feature = "board_k210") {
        unsafe {
            while TXDATA.read_volatile() & (1 << 31) != 0 {}
            (TXDATA as *mut u8).write_volatile(c as u8);
        }
    } else if cfg!(feature = "m_mode") {
        (super::BBL.mcall_console_putchar)(c);
    } else {
        sbi::console_putchar(c as usize);
    }
}

pub fn getchar() -> char {
    let c = if cfg!(feature = "board_k210") {
        unsafe {
            while RXDATA.read_volatile() & (1 << 31) == 0 {}
            (RXDATA as *const u8).read_volatile()
        }
    } else if cfg!(feature = "m_mode") {
        (super::BBL.mcall_console_getchar)() as u8
    } else {
        sbi::console_getchar() as u8
    };

    match c {
        255 => '\0',   // null
        c => c as char,
    }
}

pub fn putfmt(fmt: Arguments) {
    SerialPort.write_fmt(fmt).unwrap();
}

const TXDATA: *mut u32 = 0x38000000 as *mut u32;
const RXDATA: *mut u32 = 0x38000004 as *mut u32;
