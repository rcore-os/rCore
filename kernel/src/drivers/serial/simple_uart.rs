//! naive serial adapter driver for thinpad

use crate::util::{read, write};
use core::fmt::{Arguments, Result, Write};
use spin::Mutex;

#[derive(Debug, Clone, Copy)]
pub struct SerialPort {
    base: usize,
}

const UART_STATUS: usize = 0x0;
const UART_DATA: usize = 0x4;

const UART_STATUS_CTS: u8 = 0x1; // clear to send signal
const UART_STATUS_DR: u8 = 0x2; // data ready signal

impl SerialPort {
    fn new() -> SerialPort {
        SerialPort { base: 0 }
    }

    pub fn init(&mut self, base: usize) {
        self.base = base;
    }

    /// non-blocking version of putchar()
    pub fn putchar(&mut self, c: u8) {
        write(self.base + UART_DATA, c);
    }

    /// blocking version of getchar()
    pub fn getchar(&mut self) -> char {
        loop {
            if (read::<u8>(self.base + UART_STATUS) & UART_STATUS_DR) == 0 {
                break;
            }
        }
        let c = read::<u8>(self.base + UART_DATA);
        match c {
            255 => '\0', // null
            c => c as char,
        }
    }

    /// non-blocking version of getchar()
    pub fn getchar_option(&mut self) -> Option<char> {
        match read::<u8>(self.base + UART_STATUS) & UART_STATUS_DR {
            0 => None,
            _ => Some(read::<u8>(self.base + UART_DATA) as u8 as char),
        }
    }

    pub fn putfmt(&mut self, fmt: Arguments) {
        self.write_fmt(fmt).unwrap();
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> Result {
        for c in s.bytes() {
            match c {
                127 => {
                    self.putchar(8);
                    self.putchar(b' ');
                    self.putchar(8);
                }
                b'\n' => {
                    self.putchar(b'\r');
                    self.putchar(b'\n');
                }
                c => {
                    self.putchar(c);
                }
            }
        }
        Ok(())
    }
}

// pub static SERIAL_PORT: SerialPort = SerialPort { base: 0xa3000000 };
lazy_static! {
    pub static ref SERIAL_PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new());
}

pub fn init(base: usize) {
    SERIAL_PORT.lock().init(base);
}
