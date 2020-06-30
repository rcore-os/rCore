//! uart in bcm2837
use super::super::irq::IntcDriver;
use super::super::DRIVERS;
use super::super::IRQ_MANAGER;
use super::{super::SERIAL_DRIVERS, SerialDriver};
use crate::drivers::irq::bcm2837::BCM2837_INTC;
use crate::drivers::{DeviceType, Driver};
use crate::sync::SpinNoIrqLock as Mutex;
use alloc::string::String;
use alloc::sync::Arc;
use bcm2837::interrupt::Interrupt;
use bcm2837::mini_uart::{MiniUart, MiniUartInterruptId};

struct Bcm2837Serial {
    mu: Mutex<MiniUart>,
}

impl Driver for Bcm2837Serial {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        let mu = self.mu.lock();
        if mu.interrupt_is_pending(MiniUartInterruptId::Recive) {
            // avoid deadlock
            drop(mu);
            let c = self.read();
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
        format!("bcm2837_serial")
    }
}

impl SerialDriver for Bcm2837Serial {
    fn read(&self) -> u8 {
        self.mu.lock().read_byte()
    }

    fn write(&self, data: &[u8]) {
        let mut mu = self.mu.lock();
        for byte in data {
            mu.write_byte(*byte);
        }
    }
}

pub fn driver_init() {
    let mut mu = MiniUart::new();
    mu.init();
    let serial = Arc::new(Bcm2837Serial { mu: Mutex::new(mu) });
    DRIVERS.write().push(serial.clone());
    SERIAL_DRIVERS.write().push(serial.clone());
    BCM2837_INTC.register_local_irq(Interrupt::Aux as u8 as usize, serial);
}
