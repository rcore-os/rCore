use super::Driver;
use alloc::collections::btree_map::Entry;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct IrqManager {
    // drivers that only respond to specific irq
    mapping: BTreeMap<u32, Vec<Arc<dyn Driver>>>,
    // drivers that respond to all irqs
    all: Vec<Arc<dyn Driver>>,
}

impl IrqManager {
    pub fn new() -> IrqManager {
        IrqManager {
            mapping: BTreeMap::new(),
            all: Vec::new(),
        }
    }

    pub fn register_irq(&mut self, irq: u32, driver: Arc<dyn Driver>) {
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

    pub fn register_opt(&mut self, irq_opt: Option<u32>, driver: Arc<dyn Driver>) {
        if let Some(irq) = irq_opt {
            self.register_irq(irq, driver);
        } else {
            self.register_all(driver);
        }
    }

    pub fn deregister_irq(&mut self, irq: u32, driver: Arc<dyn Driver>) {
        if let Some(e) = self.mapping.get_mut(&irq) {
            e.retain(|d| !Arc::ptr_eq(&d, &driver));
        }
    }

    pub fn deregister_all(&mut self, driver: Arc<dyn Driver>) {
        self.all.retain(|d| !Arc::ptr_eq(&d, &driver));
    }

    pub fn try_handle_interrupt(&self, irq_opt: Option<u32>) -> bool {
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
