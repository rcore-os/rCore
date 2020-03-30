use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;

use log::*;
use virtio_drivers::{VirtIOHeader, VirtIOInput};

use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER};
use crate::sync::SpinNoIrqLock as Mutex;

struct VirtIOInputDriver(Mutex<VirtIOInput<'static>>);

impl Driver for VirtIOInputDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        let mut input = self.0.lock();
        let ack = input.ack_interrupt().expect("failed to ack interrupt");
        if ack {
            info!("mouse: {:?}", input.mouse_xy());
        }
        ack
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Input
    }

    fn get_id(&self) -> String {
        String::from("virtio_input")
    }
}

pub fn init(header: &'static mut VirtIOHeader) {
    let event_buf = Box::leak(Box::new([0u64; 32]));
    let mut input = VirtIOInput::new(header, event_buf).expect("failed to create input driver");

    let driver = Arc::new(VirtIOInputDriver(Mutex::new(input)));
    IRQ_MANAGER.write().register_all(driver.clone());
    DRIVERS.write().push(driver);
}
