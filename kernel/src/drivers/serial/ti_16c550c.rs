//! TI 16c550c serial adapter driver for malta board

#![allow(dead_code)]

use crate::util::{read, write};
use core::fmt::{Arguments, Result, Write};
use spin::Mutex;

pub struct SerialPort {
    base: usize,
}

impl SerialPort {
    fn new() -> SerialPort {
        SerialPort { base: 0 }
    }

    pub fn init(&mut self, base: usize) {
        self.base = base;
        // Turn off the FIFO
        // write(self.base + COM_FCR, 0 as u8);
        // Set speed; requires DLAB latch
        // write(self.base + COM_LCR, COM_LCR_DLAB);
        // write(self.base + COM_DLL, (115200 / 9600) as u8);
        // write(self.base + COM_DLM, 0 as u8);

        // 8 data bits, 1 stop bit, parity off; turn off DLAB latch
        // write(self.base + COM_LCR, COM_LCR_WLEN8 & !COM_LCR_DLAB);

        // No modem controls
        // write(self.base + COM_MCR, 0 as u8);
        // Enable rcv interrupts
        write(self.base + COM_INT_EN, 0x1);
    }

    /// non-blocking version of putchar()
    pub fn putchar(&mut self, c: u8) {
        write(self.base + COM_TX, c);
    }

    /// blocking version of getchar()
    pub fn getchar(&mut self) -> char {
        loop {
            if (read::<u8>(self.base + COM_LSR) & 0x01) == 0 {
                break;
            }
        }
        let c = read::<u8>(self.base + COM_RX);
        match c {
            255 => '\0', // null
            c => c as char,
        }
    }

    /// non-blocking version of getchar()
    pub fn getchar_option(&mut self) -> Option<char> {
        match read::<u8>(self.base + COM_LSR) & 0x01 {
            0 => None,
            _ => Some(read::<u8>(self.base + COM_RX) as u8 as char),
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

const COM_RX: usize = 0x00; // In:  Receive buffer (DLAB=0)
const COM_TX: usize = 0x00; // Out: Transmit buffer (DLAB=0)
const COM_INT_EN: usize = 0x08; // In:  Interrupt enable
const COM_INT_ID: usize = 0x10; // Out: Interrupt identification
const COM_LSR: usize = 0x28; // In:  Line status register

lazy_static! {
    pub static ref SERIAL_PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new());
}

pub fn init(base: usize) {
    SERIAL_PORT.lock().init(base);
}
