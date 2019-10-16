use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use lazy_static::lazy_static;
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address};
use spin::RwLock;

use crate::sync::Condvar;

#[allow(dead_code)]
pub mod block;
#[allow(dead_code)]
pub mod bus;
pub mod console;
mod device_tree;
#[allow(dead_code)]
pub mod gpu;
#[allow(dead_code)]
mod input;
pub mod irq;
pub mod mmc;
#[allow(dead_code)]
pub mod net;
mod provider;

#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Net,
    Gpu,
    Input,
    Block,
}

pub trait Driver: Send + Sync {
    // if interrupt belongs to this driver, handle it and return true
    // return false otherwise
    // irq number is provided when available
    // driver should skip handling when irq number is mismatched
    fn try_handle_interrupt(&self, irq: Option<u32>) -> bool;

    // return the correspondent device type, see DeviceType
    fn device_type(&self) -> DeviceType;

    // get unique identifier for this device
    // should be different for each instance
    fn get_id(&self) -> String;

    // Rust trait is still too restricted...
    // network related drivers should implement these
    // get mac address for this device
    fn get_mac(&self) -> EthernetAddress {
        unimplemented!("not a net driver")
    }

    // get interface name for this device
    fn get_ifname(&self) -> String {
        unimplemented!("not a net driver")
    }

    // get ip addresses
    fn get_ip_addresses(&self) -> Vec<IpCidr> {
        unimplemented!("not a net driver")
    }

    // get ipv4 address
    fn ipv4_address(&self) -> Option<Ipv4Address> {
        unimplemented!("not a net driver")
    }

    // manually trigger a poll, use it after sending packets
    fn poll(&self) {
        unimplemented!("not a net driver")
    }

    // send an ethernet frame, only use it when necessary
    fn send(&self, _data: &[u8]) -> Option<usize> {
        unimplemented!("not a net driver")
    }

    // get mac address from ip address in arp table
    fn get_arp(&self, _ip: IpAddress) -> Option<EthernetAddress> {
        unimplemented!("not a net driver")
    }

    // block related drivers should implement these
    fn read_block(&self, _block_id: usize, _buf: &mut [u8]) -> bool {
        unimplemented!("not a block driver")
    }

    fn write_block(&self, _block_id: usize, _buf: &[u8]) -> bool {
        unimplemented!("not a block driver")
    }
}

lazy_static! {
    // NOTE: RwLock only write when initializing drivers
    pub static ref DRIVERS: RwLock<Vec<Arc<dyn Driver>>> = RwLock::new(Vec::new());
    pub static ref NET_DRIVERS: RwLock<Vec<Arc<dyn Driver>>> = RwLock::new(Vec::new());
    pub static ref BLK_DRIVERS: RwLock<Vec<Arc<dyn Driver>>> = RwLock::new(Vec::new());
    pub static ref IRQ_MANAGER: RwLock<irq::IrqManager> = RwLock::new(irq::IrqManager::new());
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
}

lazy_static! {
    // Write only once at boot
    pub static ref CMDLINE: RwLock<String> = RwLock::new(String::new());
}
