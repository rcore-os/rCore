use core::fmt::{Write, Result, Arguments};

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
    // TODO: output to uart
}

pub fn getchar() -> char {
    // TODO: get char from uart
    let c = 0 as u8;

    match c {
        255 => '\0',   // null
        c => c as char,
    }
}

pub fn getchar_option() -> Option<char> {
    // TODO: get char from uart
    let c = 0 as u8;
    match c {
        -1 => None,
        c => Some(c as u8 as char),
    }
}

pub fn putfmt(fmt: Arguments) {
    SerialPort.write_fmt(fmt).unwrap();
}

const TXDATA: *mut u32 = 0x38000000 as *mut u32;
const RXDATA: *mut u32 = 0x38000004 as *mut u32;
