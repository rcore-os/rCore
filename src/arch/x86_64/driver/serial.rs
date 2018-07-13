// Copy from Redox
extern crate uart_16550;

use core::fmt::{self, Write};
use super::redox_syscall::io::{Io, Pio};
use spin::Mutex;
use self::uart_16550::SerialPort;

pub static COM1: Mutex<SerialPort> = Mutex::new(SerialPort::new(0x3F8));
pub static COM2: Mutex<SerialPort> = Mutex::new(SerialPort::new(0x2F8));

pub fn init() {
    assert_has_not_been_called!("serial::init must be called only once");

    COM1.lock().init();
    COM2.lock().init();
    use arch::interrupt::{enable_irq, consts::{IRQ_COM1, IRQ_COM2}};
    enable_irq(IRQ_COM1);
    enable_irq(IRQ_COM2);
}

pub trait SerialRead {
    fn receive(&mut self);
}

impl SerialRead for SerialPort {
    fn receive(&mut self) {
        unsafe {
            let ports = self as *mut _ as *mut [Pio<u8>; 6];
            let line_sts = &(*ports)[5];
            let data = &(*ports)[0];
            while line_sts.read() & 1 == 1 {
                let data = data.read();
                writeln!(self, "serial receive {}", data).unwrap();
                // TODO handle received data
            }
        }
    }
}
