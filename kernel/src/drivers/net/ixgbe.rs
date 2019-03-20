//! Intel 10Gb Network Adapter 82599 i.e. ixgbe network driver
//! Datasheet: https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/82599-10-gbe-controller-datasheet.pdf

use alloc::alloc::{GlobalAlloc, Layout};
use alloc::prelude::*;
use alloc::sync::Arc;
use core::mem::size_of;
use core::slice;
use core::sync::atomic::{fence, Ordering};

use alloc::collections::BTreeMap;
use bitflags::*;
use isomorphic_drivers::net::ethernet::intel::ixgbe;
use log::*;
use rcore_memory::paging::PageTable;
use rcore_memory::PAGE_SIZE;
use smoltcp::iface::*;
use smoltcp::phy::{self, Checksum, DeviceCapabilities};
use smoltcp::socket::*;
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::*;
use smoltcp::Result;
use volatile::Volatile;

use crate::memory::active_table;
use crate::net::SOCKETS;
use crate::sync::FlagsGuard;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::sync::{MutexGuard, SpinNoIrq};
use crate::HEAP_ALLOCATOR;

use super::super::{provider::Provider, DeviceType, Driver, DRIVERS, NET_DRIVERS, SOCKET_ACTIVITY};

#[derive(Clone)]
struct IXGBEDriver {
    inner: ixgbe::IXGBEDriver,
    header: usize,
    size: usize,
}

impl Drop for IXGBEDriver {
    fn drop(&mut self) {
        let _ = FlagsGuard::no_irq_region();
        let header = self.header;
        let size = self.size;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
    }
}

pub struct IXGBEInterface {
    iface: Mutex<EthernetInterface<'static, 'static, 'static, IXGBEDriver>>,
    driver: IXGBEDriver,
    ifname: String,
    irq: Option<u32>,
    id: String,
}

impl Driver for IXGBEInterface {
    fn try_handle_interrupt(&self, irq: Option<u32>) -> bool {
        if irq.is_some() && self.irq.is_some() && irq != self.irq {
            // not ours, skip it
            return false;
        }

        let handled = {
            let _ = FlagsGuard::no_irq_region();
            let header = self.driver.header;
            let size = self.driver.size;
            if let None = active_table().get_entry(header) {
                let mut current_addr = header;
                while current_addr < header + size {
                    active_table().map_if_not_exists(current_addr, current_addr);
                    current_addr = current_addr + PAGE_SIZE;
                }
            }

            self.driver.inner.try_handle_interrupt()
        };

        if handled {
            let timestamp = Instant::from_millis(crate::trap::uptime_msec() as i64);
            let mut sockets = SOCKETS.lock();
            match self.iface.lock().poll(&mut sockets, timestamp) {
                Ok(_) => {
                    SOCKET_ACTIVITY.notify_all();
                }
                Err(err) => {
                    debug!("poll got err {}", err);
                }
            }
        }

        return handled;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }

    fn get_id(&self) -> String {
        self.ifname.clone()
    }

    fn get_mac(&self) -> EthernetAddress {
        self.iface.lock().ethernet_addr()
    }

    fn get_ifname(&self) -> String {
        self.ifname.clone()
    }

    fn ipv4_address(&self) -> Option<Ipv4Address> {
        self.iface.lock().ipv4_address()
    }

    fn poll(&self) {
        let timestamp = Instant::from_millis(crate::trap::uptime_msec() as i64);
        let mut sockets = SOCKETS.lock();
        match self.iface.lock().poll(&mut sockets, timestamp) {
            Ok(_) => {
                SOCKET_ACTIVITY.notify_all();
            }
            Err(err) => {
                debug!("poll got err {}", err);
            }
        }
    }
}
pub struct IXGBERxToken(Vec<u8>);
pub struct IXGBETxToken(IXGBEDriver);

impl<'a> phy::Device<'a> for IXGBEDriver {
    type RxToken = IXGBERxToken;
    type TxToken = IXGBETxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let _ = FlagsGuard::no_irq_region();
        let header = self.header;
        let size = self.size;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
        if self.inner.can_send() {
            if let Some(data) = self.inner.recv() {
                Some((IXGBERxToken(data), IXGBETxToken(self.clone())))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        let _ = FlagsGuard::no_irq_region();
        let header = self.header;
        let size = self.size;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
        if self.inner.can_send() {
            Some(IXGBETxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = ixgbe::IXGBEDriver::get_mtu(); // max MTU
        caps.max_burst_size = Some(256);
        // IP Rx checksum is offloaded with RXCSUM
        caps.checksum.ipv4 = Checksum::Tx;
        caps
    }
}

impl phy::RxToken for IXGBERxToken {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> Result<R>,
    {
        f(&self.0)
    }
}

impl phy::TxToken for IXGBETxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let _ = FlagsGuard::no_irq_region();
        let header = self.0.header;
        let size = self.0.size;
        if let None = active_table().get_entry(header) {
            let mut current_addr = header;
            while current_addr < header + size {
                active_table().map_if_not_exists(current_addr, current_addr);
                current_addr = current_addr + PAGE_SIZE;
            }
        }
        let mut buffer = [0u8; ixgbe::IXGBEDriver::get_mtu()];
        let result = f(&mut buffer[..len]);
        if result.is_ok() {
            (self.0).inner.send(&buffer[..len]);
        }
        result
    }
}

pub fn ixgbe_init(
    name: String,
    irq: Option<u32>,
    header: usize,
    size: usize,
) -> Arc<IXGBEInterface> {
    let _ = FlagsGuard::no_irq_region();
    if let None = active_table().get_entry(header) {
        let mut current_addr = header;
        while current_addr < header + size {
            active_table().map_if_not_exists(current_addr, current_addr);
            current_addr = current_addr + PAGE_SIZE;
        }
    }
    let ixgbe = ixgbe::IXGBEDriver::init(Provider::new(), header, size);
    let ethernet_addr = EthernetAddress::from_bytes(&ixgbe.get_mac().as_bytes());

    let net_driver = IXGBEDriver {
        inner: ixgbe,
        header,
        size,
    };

    let ip_addrs = [IpCidr::new(IpAddress::v4(10, 0, 0, 2), 24)];
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let iface = EthernetInterfaceBuilder::new(net_driver.clone())
        .ethernet_addr(ethernet_addr)
        .ip_addrs(ip_addrs)
        .neighbor_cache(neighbor_cache)
        .finalize();

    info!("ixgbe: interface {} up", &name);

    let ixgbe_iface = IXGBEInterface {
        iface: Mutex::new(iface),
        driver: net_driver.clone(),
        ifname: name.clone(),
        id: name,
        irq,
    };

    let driver = Arc::new(ixgbe_iface);
    DRIVERS.write().push(driver.clone());
    NET_DRIVERS.write().push(driver.clone());
    driver
}
