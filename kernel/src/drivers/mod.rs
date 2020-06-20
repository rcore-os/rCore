use crate::sync::Condvar;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use rcore_fs::dev::{self, BlockDevice, DevError};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use spin::RwLock;

pub use block::BlockDriver;
pub use net::NetDriver;
pub use rtc::RtcDriver;
pub use serial::SerialDriver;

/// Block device
pub mod block;
/// Bus controller
pub mod bus;
/// Character console
pub mod console;
/// Device tree
pub mod device_tree;
/// Display controller
pub mod gpu;
/// Mouse device
pub mod input;
/// Interrupt controller
pub mod irq;
/// Network controller
pub mod net;
/// For isomorphic_drivers
pub mod provider;
/// Real time clock
pub mod rtc;
/// Serial port
pub mod serial;

#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Net,
    Gpu,
    Input,
    Block,
    Rtc,
    Serial,
}

pub trait Driver: Send + Sync {
    // if interrupt belongs to this driver, handle it and return true
    // return false otherwise
    // irq number is provided when available
    // driver should skip handling when irq number is mismatched
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool;

    // return the correspondent device type, see DeviceType
    fn device_type(&self) -> DeviceType;

    // get unique identifier for this device
    // should be different for each instance
    fn get_id(&self) -> String;

    // trait casting
    fn as_net(&self) -> Option<&dyn NetDriver> {
        None
    }

    fn as_block(&self) -> Option<&dyn BlockDriver> {
        None
    }

    fn as_rtc(&self) -> Option<&dyn RtcDriver> {
        None
    }
}

lazy_static! {
    // NOTE: RwLock only write when initializing drivers
    pub static ref DRIVERS: RwLock<Vec<Arc<dyn Driver>>> = RwLock::new(Vec::new());
    pub static ref NET_DRIVERS: RwLock<Vec<Arc<dyn NetDriver>>> = RwLock::new(Vec::new());
    pub static ref BLK_DRIVERS: RwLock<Vec<Arc<dyn BlockDriver>>> = RwLock::new(Vec::new());
    pub static ref RTC_DRIVERS: RwLock<Vec<Arc<dyn RtcDriver>>> = RwLock::new(Vec::new());
    pub static ref SERIAL_DRIVERS: RwLock<Vec<Arc<dyn SerialDriver>>> = RwLock::new(Vec::new());
    pub static ref IRQ_MANAGER: RwLock<irq::IrqManager> = RwLock::new(irq::IrqManager::new());
}

pub struct BlockDriverWrapper(pub Arc<dyn BlockDriver>);

impl BlockDevice for BlockDriverWrapper {
    const BLOCK_SIZE_LOG2: u8 = 9; // 512
    fn read_at(&self, block_id: usize, buf: &mut [u8]) -> dev::Result<()> {
        match self.0.read_block(block_id, buf) {
            true => Ok(()),
            false => Err(DevError),
        }
    }

    fn write_at(&self, block_id: usize, buf: &[u8]) -> dev::Result<()> {
        match self.0.write_block(block_id, buf) {
            true => Ok(()),
            false => Err(DevError),
        }
    }

    fn sync(&self) -> dev::Result<()> {
        Ok(())
    }
}

lazy_static! {
    pub static ref SOCKET_ACTIVITY: Condvar = Condvar::new();
}

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64", target_arch = "mips"))]
pub fn init(dtb: usize) {
    device_tree::init(dtb);
}

#[cfg(target_arch = "x86_64")]
pub fn init() {
    bus::pci::init();
    rtc::rtc_cmos::init();
}

#[cfg(target_arch = "x86_64")]
pub fn early_init() {
    serial::com::init();
}

lazy_static! {
    // Write only once at boot
    pub static ref CMDLINE: RwLock<String> = RwLock::new(String::new());
}
