use super::Driver;
use crate::arch::interrupt::enable_irq;
use alloc::collections::btree_map::Entry;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub mod plic;

// Irq manager
pub struct IrqManager {
    // is root manager?
    root: bool,
    // drivers that only respond to specific irq
    mapping: BTreeMap<usize, Vec<Arc<dyn Driver>>>,
    // drivers that respond to all irqs
    all: Vec<Arc<dyn Driver>>,
}

impl IrqManager {
    pub fn new(root: bool) -> IrqManager {
        IrqManager {
            root,
            mapping: BTreeMap::new(),
            all: Vec::new(),
        }
    }

    pub fn register_irq(&mut self, irq: usize, driver: Arc<dyn Driver>) {
        // for root manager, enable irq in arch
        // for other interrupt controllers, enable irq before calling this function
        if self.root {
            enable_irq(irq);
        }
        match self.mapping.entry(irq) {
            Entry::Occupied(mut e) => {
                e.get_mut().push(driver);
            }
            Entry::Vacant(e) => {
                let mut v = Vec::new();
                v.push(driver);
                e.insert(v);
            }
        }
    }

    pub fn register_all(&mut self, driver: Arc<dyn Driver>) {
        self.all.push(driver);
    }

    pub fn register_opt(&mut self, irq_opt: Option<usize>, driver: Arc<dyn Driver>) {
        if let Some(irq) = irq_opt {
            self.register_irq(irq, driver);
        } else {
            self.register_all(driver);
        }
    }

    pub fn deregister_irq(&mut self, irq: usize, driver: Arc<dyn Driver>) {
        if let Some(e) = self.mapping.get_mut(&irq) {
            e.retain(|d| !Arc::ptr_eq(&d, &driver));
        }
    }

    pub fn deregister_all(&mut self, driver: Arc<dyn Driver>) {
        self.all.retain(|d| !Arc::ptr_eq(&d, &driver));
    }

    pub fn try_handle_interrupt(&self, irq_opt: Option<usize>) -> bool {
        if let Some(irq) = irq_opt {
            if let Some(e) = self.mapping.get(&irq) {
                for dri in e.iter() {
                    if dri.try_handle_interrupt(Some(irq)) {
                        return true;
                    }
                }
            }
        }

        for dri in self.all.iter() {
            if dri.try_handle_interrupt(irq_opt) {
                return true;
            }
        }
        false
    }
}

// interrupt controller
pub trait IntcDriver: Driver {
    /// Register interrupt controller local irq
    fn register_local_irq(&self, irq: usize, driver: Arc<dyn Driver>);
}
