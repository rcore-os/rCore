use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::port::Port;

use crate::arch::interrupt::{consts, enable_irq};

pub static COM1: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x3F8) });
pub static COM2: Mutex<SerialPort> = Mutex::new(unsafe { SerialPort::new(0x2F8) });

pub fn init() {
    COM1.lock().init();
    COM2.lock().init();
    enable_irq(consts::COM1);
    enable_irq(consts::COM2);
    info!("serial: init end");
}

pub trait SerialRead {
    fn receive(&mut self) -> u8;
}

impl SerialRead for SerialPort {
    fn receive(&mut self) -> u8 {
        unsafe {
            let ports = self as *mut _ as *mut [Port<u8>; 6];
            let data = &mut (*ports)[0];
            data.read()
        }
    }
}
