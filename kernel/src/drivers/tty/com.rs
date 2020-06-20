//! COM ports in x86
use super::super::DRIVERS;
use super::super::IRQ_MANAGER;
use super::{super::TTY_DRIVERS, TtyDriver};
use crate::{
    drivers::{DeviceType, Driver},
    sync::SpinNoIrqLock as Mutex,
};
use alloc::string::String;
use alloc::sync::Arc;
use uart_16550::SerialPort;

pub const COM2: usize = 3;
pub const COM1: usize = 4;

struct COM {
    port: Mutex<SerialPort>,
    base: u16,
}

impl COM {
    fn new(base: u16) -> COM {
        let port = Mutex::new(unsafe { SerialPort::new(base) });
        port.lock().init();
        COM { port, base }
    }
}

impl Driver for COM {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        false
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Tty
    }

    fn get_id(&self) -> String {
        format!("com_{}", self.base)
    }
}

impl TtyDriver for COM {
    fn read(&self) -> u8 {
        self.port.lock().receive()
    }

    fn write(&self, data: &[u8]) {
        let mut port = self.port.lock();
        for byte in data {
            port.send(*byte);
        }
    }
}

pub fn init() {
    add(0x3F8, COM1);
    add(0x2F8, COM2);
}

fn add(base: u16, irq: usize) {
    let com = Arc::new(COM::new(base));
    DRIVERS.write().push(com.clone());
    TTY_DRIVERS.write().push(com.clone());
    IRQ_MANAGER.write().register_irq(irq, com);
}
