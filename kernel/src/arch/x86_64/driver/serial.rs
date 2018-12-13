// Copy from Redox

use core::fmt::{self, Write};
use x86_64::instructions::port::Port;
use spin::Mutex;
use uart_16550::SerialPort;
use once::*;

pub static COM1: Mutex<SerialPort> = Mutex::new(SerialPort::new(0x3F8));
pub static COM2: Mutex<SerialPort> = Mutex::new(SerialPort::new(0x2F8));

pub fn init() {
    assert_has_not_been_called!("serial::init must be called only once");

    COM1.lock().init();
    COM2.lock().init();
    use crate::arch::interrupt::{enable_irq, consts::{IRQ_COM1, IRQ_COM2}};
    enable_irq(IRQ_COM1);
    enable_irq(IRQ_COM2);
}

pub trait SerialRead {
    fn receive(&mut self) -> u8;
}

impl SerialRead for SerialPort {
    fn receive(&mut self) -> u8 {
        unsafe {
            let ports = self as *mut _ as *mut [Port<u8>; 6];
            let line_sts = &(*ports)[5];
            let data = &(*ports)[0];
            data.read()
        }
    }
}
