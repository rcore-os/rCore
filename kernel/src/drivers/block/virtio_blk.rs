use alloc::string::String;
use alloc::sync::Arc;

use log::*;
use virtio_drivers::{VirtIOBlk, VirtIOHeader};

use super::super::{DeviceType, Driver, BLK_DRIVERS, DRIVERS, IRQ_MANAGER};
use crate::memory::phys_to_virt;
use crate::sync::SpinNoIrqLock as Mutex;

struct VirtIOBlkDriver(Mutex<VirtIOBlk<'static>>);

impl Driver for VirtIOBlkDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        self.0.lock().ack_interrupt()
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> String {
        format!("virtio_block")
    }

    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        self.0.lock().read_block(block_id, buf).is_ok()
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        self.0.lock().write_block(block_id, buf).is_ok()
    }
}

pub fn init(header: &'static mut VirtIOHeader) {
    let blk = VirtIOBlk::new(header).expect("failed to init blk driver");
    let driver = Arc::new(VirtIOBlkDriver(Mutex::new(blk)));
    DRIVERS.write().push(driver.clone());
    IRQ_MANAGER.write().register_all(driver.clone());
    BLK_DRIVERS.write().push(driver);
}
