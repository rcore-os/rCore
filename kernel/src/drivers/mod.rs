use alloc::prelude::*;
use alloc::sync::Arc;

use lazy_static::lazy_static;
use smoltcp::wire::{EthernetAddress, Ipv4Address};
use smoltcp::socket::SocketSet;
use spin::RwLock;

use crate::sync::{Condvar, MutexGuard, SpinNoIrq};
use self::block::virtio_blk::VirtIOBlkDriver;

mod device_tree;
pub mod bus;
pub mod net;
pub mod block;
mod gpu;
mod input;

#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Net,
    Gpu,
    Input,
    Block
}

pub trait Driver : Send + Sync {
    // if interrupt belongs to this driver, handle it and return true
    // return false otherwise
    // irq number is provided when available
    // driver should skip handling when irq number is mismatched
    fn try_handle_interrupt(&self, irq: Option<u32>) -> bool;

    // return the correspondent device type, see DeviceType
    fn device_type(&self) -> DeviceType;
}

pub trait NetDriver : Driver {
    // get mac address for this device
    fn get_mac(&self) -> EthernetAddress;

    // get interface name for this device
    fn get_ifname(&self) -> String;

    // get ipv4 address
    fn ipv4_address(&self) -> Option<Ipv4Address>;

    // get sockets
    fn sockets(&self) -> MutexGuard<SocketSet<'static, 'static, 'static>, SpinNoIrq>;

    // manually trigger a poll, use it after sending packets
    fn poll(&self);
}


lazy_static! {
    // NOTE: RwLock only write when initializing drivers
    pub static ref DRIVERS: RwLock<Vec<Arc<Driver>>> = RwLock::new(Vec::new());
    pub static ref NET_DRIVERS: RwLock<Vec<Arc<NetDriver>>> = RwLock::new(Vec::new());
    pub static ref BLK_DRIVERS: RwLock<Vec<Arc<VirtIOBlkDriver>>> = RwLock::new(Vec::new());
}

lazy_static!{
    pub static ref SOCKET_ACTIVITY: Condvar = Condvar::new();
}

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub fn init(dtb: usize) {
    device_tree::init(dtb);
}

#[cfg(target_arch = "x86_64")]
pub fn init() {
    bus::pci::init();
}