//! Driver for AHCI
//!
//! Spec: https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/serial-ata-ahci-spec-rev1-3-1.pdf

use alloc::string::String;
use alloc::sync::Arc;

use isomorphic_drivers::block::ahci::{AHCI, BLOCK_SIZE};

use crate::drivers::provider::Provider;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{DeviceType, Driver, BLK_DRIVERS, DRIVERS};

pub struct AHCIDriver(Mutex<AHCI<Provider>>);

impl Driver for AHCIDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        false
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> String {
        format!("ahci")
    }

    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        let mut driver = self.0.lock();
        driver.read_block(block_id, buf);
        true
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        if buf.len() < BLOCK_SIZE {
            return false;
        }
        let mut driver = self.0.lock();
        driver.write_block(block_id, buf);
        true
    }
}

pub fn init(_irq: Option<u32>, header: usize, size: usize) -> Option<Arc<AHCIDriver>> {
    if let Some(ahci) = AHCI::new(header, size) {
        let driver = Arc::new(AHCIDriver(Mutex::new(ahci)));
        DRIVERS.write().push(driver.clone());
        BLK_DRIVERS.write().push(driver.clone());
        Some(driver)
    } else {
        None
    }
}
