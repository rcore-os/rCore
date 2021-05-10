use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;

use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER, SERIAL_DRIVERS};
use crate::drivers::device_tree::{DEVICE_TREE_INTC, DEVICE_TREE_REGISTRY};
use crate::{
    drivers::{BlockDriver, NetDriver},
    sync::SpinNoIrqLock as Mutex,
};
use device_tree::Node;
use log::*;
use virtio_drivers::VirtIOConsole;
use virtio_drivers::{VirtIOHeader, VirtIOInput};

struct VirtIOConsoleDriver(Mutex<VirtIOConsole<'static>>);
pub fn init(dt: &Node, header: &'static mut VirtIOHeader) {
    let mut console = VirtIOConsole::new(header).expect("failed to create virtio console");
    let driver = Arc::new(VirtIOConsoleDriver(Mutex::new(console)));
    let irq_opt = dt.prop_u32("interrupts").ok().map(|irq| irq as usize);
    let mut found = false;
    if let Ok(intc) = dt.prop_u32("interrupt-parent") {
        if let Some(irq) = irq_opt {
            if let Some(manager) = DEVICE_TREE_INTC.write().get_mut(&intc) {
                manager.register_local_irq(irq, driver.clone());
                info!("registered virtio_console to intc");
                found = true;
            }
        }
    }
    if !found {
        info!("registered virtio_console to root");
        IRQ_MANAGER.write().register_opt(irq_opt, driver.clone());
    }
    SERIAL_DRIVERS.write().push(driver);
}
impl Driver for VirtIOConsoleDriver {
    fn try_handle_interrupt(&self, _irq: Option<usize>) -> bool {
        let mut console = self.0.lock();
        let ack = console.ack_interrupt().expect("failed to ack interrupt");
        if ack {
            super::SERIAL_ACTIVITY.notify_all();
        }
        ack
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Serial
    }

    fn get_id(&self) -> String {
        String::from("virtio_console")
    }

    fn as_net(&self) -> Option<&dyn NetDriver> {
        None
    }

    fn as_block(&self) -> Option<&dyn BlockDriver> {
        None
    }
}

impl crate::drivers::serial::SerialDriver for VirtIOConsoleDriver {
    fn read(&self) -> u8 {
        let mut console = self.0.lock();
        console.recv(true).unwrap().unwrap_or(0)
    }

    fn write(&self, data: &[u8]) {
        let mut console = self.0.lock();
        for byte in data {
            console.send(*byte).unwrap();
        }
    }
    fn try_read(&self) -> Option<u8> {
        let mut console = self.0.lock();
        console.recv(true).unwrap()
    }
}
