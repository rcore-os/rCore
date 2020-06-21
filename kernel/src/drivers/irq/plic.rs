//! RISC-V plic

use super::super::DRIVERS;
use super::{super::IRQ_MANAGER, IntcDriver, IrqManager};
use crate::drivers::{
    device_tree::DEVICE_TREE_INTC, device_tree::DEVICE_TREE_REGISTRY, DeviceType, Driver,
};
use crate::memory::phys_to_virt;
use crate::{sync::SpinNoIrqLock as Mutex, util::read, util::write};
use alloc::string::String;
use alloc::sync::Arc;
use device_tree::Node;

pub struct Plic {
    base: usize,
    manager: Mutex<IrqManager>,
}

impl Driver for Plic {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        // TODO: support more than 32 irqs
        let pending: u32 = read(self.base + 0x1000);
        if pending != 0 {
            let claim: u32 = read(self.base + 0x201004);
            info!("pending {:#x} claim {:#x}", pending, claim);
            let manager = self.manager.lock();
            let res = manager.try_handle_interrupt(Some(claim as usize));
            // complete
            write(self.base + 0x201004, claim);
            res
        } else {
            false
        }
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Intc
    }

    fn get_id(&self) -> String {
        format!("plic_{}", self.base)
    }
}

impl IntcDriver for Plic {
    /// Register interrupt controller local irq
    fn register_local_irq(&self, irq: usize, driver: Arc<dyn Driver>) {
        let mut manager = self.manager.lock();
        manager.register_irq(irq, driver);
    }
}

pub const SupervisorExternal: usize = usize::MAX / 2 + 1 + 8;

pub fn init_dt(dt: &Node) {
    let addr = dt.prop_u64("reg").unwrap() as usize;
    let phandle = dt.prop_u32("phandle").unwrap();
    info!("Found riscv plic at {:#x}", addr);
    let base = phys_to_virt(addr);
    let plic = Arc::new(Plic {
        base,
        manager: Mutex::new(IrqManager::new(false)),
    });
    DRIVERS.write().push(plic.clone());
    // register under root irq manager
    IRQ_MANAGER
        .write()
        .register_irq(SupervisorExternal, plic.clone());
    // register interrupt controller
    DEVICE_TREE_INTC.write().insert(phandle, plic);
}

pub fn driver_init() {
    DEVICE_TREE_REGISTRY.write().insert("riscv,plic0", init_dt);
}
