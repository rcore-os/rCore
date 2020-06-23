//! BCM2836 interrupt

use super::super::DRIVERS;
use super::{super::IRQ_MANAGER, IntcDriver, IrqManager};
use crate::drivers::{
    device_tree::DEVICE_TREE_INTC, device_tree::DEVICE_TREE_REGISTRY, DeviceType, Driver,
};
use crate::memory::phys_to_virt;
use crate::{sync::SpinNoIrqLock as Mutex, util::read, util::write};
use alloc::string::String;
use alloc::sync::Arc;
use bcm2837::interrupt::Controller;
use bcm2837::interrupt::Interrupt;

pub struct Bcm2837Intc {
    manager: Mutex<IrqManager>,
}

impl Driver for Bcm2837Intc {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        let mut res = false;
        let manager = self.manager.lock();
        for intr in Controller::new().pending_interrupts() {
            res |= manager.try_handle_interrupt(Some(intr as usize));
        }
        res
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Intc
    }

    fn get_id(&self) -> String {
        format!("bcm2837_intc")
    }
}

impl IntcDriver for Bcm2837Intc {
    /// Register interrupt controller local irq
    fn register_local_irq(&self, irq: usize, driver: Arc<dyn Driver>) {
        // enable irq
        use bcm2837::interrupt::Interrupt::*;
        let intr = match irq {
            _ if irq == Timer1 as usize => Timer1,
            _ if irq == Aux as usize => Aux,
            _ => todo!(),
        };
        Controller::new().enable(intr);
        let mut manager = self.manager.lock();
        manager.register_irq(irq, driver);
    }
}

// singleton
lazy_static! {
    pub static ref BCM2837_INTC: Arc<Bcm2837Intc> = init();
}

fn init() -> Arc<Bcm2837Intc> {
    info!("Init bcm2837 interrupt controller");
    let intc = Arc::new(Bcm2837Intc {
        manager: Mutex::new(IrqManager::new(false)),
    });
    DRIVERS.write().push(intc.clone());
    // register under root irq manager
    // 0x10002: from lower el, irq
    IRQ_MANAGER.write().register_irq(0x10002, intc.clone());
    // 0x10001: from current el, irq
    IRQ_MANAGER.write().register_irq(0x10001, intc.clone());
    intc
}

pub fn driver_init() {
    lazy_static::initialize(&BCM2837_INTC);
}
