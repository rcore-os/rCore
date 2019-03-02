use alloc::prelude::*;
use core::any::Any;

use lazy_static::lazy_static;
use smoltcp::wire::EthernetAddress;
use smoltcp::socket::SocketSet;

use crate::sync::SpinNoIrqLock;

mod device_tree;
pub mod bus;
pub mod net;
pub mod block;
mod gpu;
mod input;

pub enum DeviceType {
    Net,
    Gpu,
    Input,
    Block
}

pub trait Driver : Send {
    // if interrupt belongs to this driver, handle it and return true
    // return false otherwise
    fn try_handle_interrupt(&mut self) -> bool;

    // return the correspondent device type, see DeviceType
    fn device_type(&self) -> DeviceType;
}

pub trait NetDriver : Send {
    // get mac address for this device
    fn get_mac(&self) -> EthernetAddress;

    // get interface name for this device
    fn get_ifname(&self) -> String;

    fn poll(&mut self, socket: &mut SocketSet) -> Option<bool>;
}


lazy_static! {
    pub static ref DRIVERS: SpinNoIrqLock<Vec<Box<Driver>>> = SpinNoIrqLock::new(Vec::new());
}

lazy_static! {
    pub static ref NET_DRIVERS: SpinNoIrqLock<Vec<Box<NetDriver>>> = SpinNoIrqLock::new(Vec::new());
}

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
pub fn init(dtb: usize) {
    device_tree::init(dtb);
}

#[cfg(target_arch = "x86_64")]
pub fn init() {
    bus::pci::init();
}