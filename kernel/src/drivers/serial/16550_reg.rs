//! 16550 serial adapter driver for malta board

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
        write(self.base + COM_FCR, 0 as u8);
        // Set speed; requires DLAB latch
        write(self.base + COM_LCR, COM_LCR_DLAB);
        write(self.base + COM_DLL, (115200 / 9600) as u8);
        write(self.base + COM_DLM, 0 as u8);

        // 8 data bits, 1 stop bit, parity off; turn off DLAB latch
        write(self.base + COM_LCR, COM_LCR_WLEN8 & !COM_LCR_DLAB);

        // No modem controls
        write(self.base + COM_MCR, 0 as u8);
        // Enable rcv interrupts
        write(self.base + COM_IER, COM_IER_RDI);
    }

    /// non-blocking version of putchar()
    pub fn putchar(&mut self, c: u8) {
        write(self.base + COM_TX, c);
    }

    /// blocking version of getchar()
    pub fn getchar(&mut self) -> char {
        loop {
            if (read::<u8>(self.base + COM_LSR) & COM_LSR_DATA) == 0 {
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
        match read::<u8>(self.base + COM_LSR) & COM_LSR_DATA {
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

const COM_RX: usize = 0; // In:  Receive buffer (DLAB=0)
const COM_TX: usize = 0; // Out: Transmit buffer (DLAB=0)
const COM_DLL: usize = 0; // Out: Divisor Latch Low (DLAB=1)
const COM_DLM: usize = 1; // Out: Divisor Latch High (DLAB=1)
const COM_IER: usize = 1; // Out: Interrupt Enable Register
const COM_IER_RDI: u8 = 0x01; // Enable receiver data interrupt
const COM_IIR: usize = 2; // In:  Interrupt ID Register
const COM_FCR: usize = 2; // Out: FIFO Control Register
const COM_LCR: usize = 3; // Out: Line Control Register
const COM_LCR_DLAB: u8 = 0x80; // Divisor latch access bit
const COM_LCR_WLEN8: u8 = 0x03; // Wordlength: 8 bits
const COM_MCR: usize = 4; // Out: Modem Control Register
const COM_MCR_RTS: u8 = 0x02; // RTS complement
const COM_MCR_DTR: u8 = 0x01; // DTR complement
const COM_MCR_OUT2: u8 = 0x08; // Out2 complement
const COM_LSR: usize = 5; // In:  Line Status Register
const COM_LSR_DATA: u8 = 0x01; // Data available
const COM_LSR_TXRDY: u8 = 0x20; // Transmit buffer avail
const COM_LSR_TSRE: u8 = 0x40; // Transmitter off

lazy_static! {
    pub static ref SERIAL_PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new());
}

pub fn init(base: usize) {
    SERIAL_PORT.lock().init(base);
}
