//! 16550 serial adapter driver for malta board

use super::SerialDriver;
use crate::drivers::IRQ_MANAGER;
use crate::drivers::SERIAL_DRIVERS;
use crate::drivers::{DeviceType, Driver, DRIVERS};
use crate::sync::SpinLock as Mutex;
use crate::util::{read, write};
use alloc::{string::String, sync::Arc};
use core::fmt::{Arguments, Result, Write};

pub struct SerialPort {
    base: usize,
}

impl Driver for SerialPort {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        if let Some(c) = self.getchar_option() {
            crate::trap::serial(c);
            true
        } else {
            false
        }
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Serial
    }

    fn get_id(&self) -> String {
        format!("com_{}", self.base)
    }
}

impl SerialPort {
    fn new(base: usize) -> SerialPort {
        let mut res = SerialPort { base: 0 };
        res.init(base);
        res
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
    pub fn putchar(&self, c: u8) {
        write(self.base + COM_TX, c);
    }

    /// blocking version of getchar()
    pub fn getchar(&mut self) -> u8 {
        loop {
            if (read::<u8>(self.base + COM_LSR) & COM_LSR_DATA) == 0 {
                break;
            }
        }
        let c = read::<u8>(self.base + COM_RX);
        match c {
            255 => b'\0', // null
            c => c,
        }
    }

    /// non-blocking version of getchar()
    pub fn getchar_option(&self) -> Option<u8> {
        match read::<u8>(self.base + COM_LSR) & COM_LSR_DATA {
            0 => None,
            _ => Some(read::<u8>(self.base + COM_RX) as u8),
        }
    }
}

impl SerialDriver for SerialPort {
    fn read(&self) -> u8 {
        self.getchar_option().unwrap_or(0)
    }

    fn write(&self, data: &[u8]) {
        for byte in data {
            self.putchar(*byte);
        }
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

pub fn init(irq: Option<usize>, base: usize) {
    info!("Init uart16550 at {:#x}", base);
    let com = Arc::new(SerialPort::new(base));
    DRIVERS.write().push(com.clone());
    SERIAL_DRIVERS.write().push(com.clone());
    IRQ_MANAGER.write().register_opt(irq, com);
}
