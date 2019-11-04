//! Intel 10Gb Network Adapter 82599 i.e. ixgbe network driver
//! Datasheet: https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/82599-10-gbe-controller-datasheet.pdf

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use alloc::collections::BTreeMap;
use isomorphic_drivers::net::ethernet::intel::ixgbe;
use log::*;
use smoltcp::iface::*;
use smoltcp::phy::{self, Checksum, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use smoltcp::wire::*;
use smoltcp::Result;

use crate::net::SOCKETS;
use crate::sync::FlagsGuard;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{
    provider::Provider, DeviceType, Driver, DRIVERS, IRQ_MANAGER, NET_DRIVERS, SOCKET_ACTIVITY,
};

#[derive(Clone)]
struct IXGBEDriver {
    inner: Arc<Mutex<ixgbe::IXGBE<Provider>>>,
    header: usize,
    size: usize,
    mtu: usize,
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
            self.driver.inner.lock().try_handle_interrupt()
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

    // get ip addresses
    fn get_ip_addresses(&self) -> Vec<IpCidr> {
        Vec::from(self.iface.lock().ip_addrs())
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

    fn send(&self, data: &[u8]) -> Option<usize> {
        self.driver.inner.lock().send(&data);
        Some(data.len())
    }

    fn get_arp(&self, ip: IpAddress) -> Option<EthernetAddress> {
        let iface = self.iface.lock();
        let cache = iface.neighbor_cache();
        cache.lookup_pure(&ip, Instant::from_millis(0))
    }
}
pub struct IXGBERxToken(Vec<u8>);
pub struct IXGBETxToken(IXGBEDriver);

impl<'a> phy::Device<'a> for IXGBEDriver {
    type RxToken = IXGBERxToken;
    type TxToken = IXGBETxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let _ = FlagsGuard::no_irq_region();
        if self.inner.lock().can_send() {
            if let Some(data) = self.inner.lock().recv() {
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
        if self.inner.lock().can_send() {
            Some(IXGBETxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        // do not use max MTU by default
        //caps.max_transmission_unit = ixgbe::IXGBEDriver::get_mtu(); // max MTU
        caps.max_transmission_unit = self.mtu;
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
        let mut buffer = [0u8; ixgbe::IXGBE::<Provider>::get_mtu()];
        let result = f(&mut buffer[..len]);
        if result.is_ok() {
            self.0.inner.lock().send(&buffer[..len]);
        }
        result
    }
}

pub fn ixgbe_init(
    name: String,
    irq: Option<u32>,
    header: usize,
    size: usize,
    index: usize,
) -> Arc<IXGBEInterface> {
    let _ = FlagsGuard::no_irq_region();
    let mut ixgbe = ixgbe::IXGBE::new(header, size);
    ixgbe.enable_irq();

    let ethernet_addr = EthernetAddress::from_bytes(&ixgbe.get_mac().as_bytes());

    let net_driver = IXGBEDriver {
        inner: Arc::new(Mutex::new(ixgbe)),
        header,
        size,
        mtu: 1500,
    };

    let ip_addrs = [IpCidr::new(IpAddress::v4(10, 0, index as u8, 2), 24)];
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let routes = Routes::new(BTreeMap::new());
    let iface = EthernetInterfaceBuilder::new(net_driver.clone())
        .ethernet_addr(ethernet_addr)
        .ip_addrs(ip_addrs)
        .neighbor_cache(neighbor_cache)
        .routes(routes)
        .finalize();

    info!("ixgbe interface {} up with addr 10.0.{}.2/24", name, index);

    let ixgbe_iface = IXGBEInterface {
        iface: Mutex::new(iface),
        driver: net_driver.clone(),
        ifname: name.clone(),
        id: name,
        irq,
    };

    let driver = Arc::new(ixgbe_iface);
    IRQ_MANAGER.write().register_opt(irq, driver.clone());
    DRIVERS.write().push(driver.clone());
    NET_DRIVERS.write().push(driver.clone());
    driver
}
