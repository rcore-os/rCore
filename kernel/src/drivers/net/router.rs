//! rCore Router Driver

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use smoltcp::iface::*;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::*;
use smoltcp::Result;

use rcore_memory::PAGE_SIZE;

use crate::drivers::provider::Provider;
use crate::net::SOCKETS;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{DeviceType, Driver, DRIVERS, NET_DRIVERS, SOCKET_ACTIVITY};
use crate::consts::{KERNEL_OFFSET, MEMORY_END, MEMORY_OFFSET};

const AXI_STREAM_FIFO_ISR: *mut u32 = (KERNEL_OFFSET + 0x1820_0000) as *mut u32;
const AXI_STREAM_FIFO_IER: *mut u32 = (KERNEL_OFFSET + 0x1820_0004) as *mut u32;
const AXI_STREAM_FIFO_RDFR: *mut u32 = (KERNEL_OFFSET + 0x1820_0018) as *mut u32;
const AXI_STREAM_FIFO_RDFO: *mut u32 = (KERNEL_OFFSET + 0x1820_001C) as *mut u32;
const AXI_STREAM_FIFO_RDFD: *mut u32 = (KERNEL_OFFSET + 0x1820_0020) as *mut u32;
const AXI_STREAM_FIFO_RLR: *mut u32 = (KERNEL_OFFSET + 0x1820_0024) as *mut u32;
const AXI_STREAM_FIFO_RDR: *mut u32 = (KERNEL_OFFSET + 0x1820_0030) as *mut u32;

const AXI_STREAM_FIFO_TDR: *mut u32 = (KERNEL_OFFSET + 0x1820_002C) as *mut u32;
const AXI_STREAM_FIFO_TDFD: *mut u32 = (KERNEL_OFFSET + 0x1820_0010) as *mut u32;
const AXI_STREAM_FIFO_TLR: *mut u32 = (KERNEL_OFFSET + 0x1820_0014) as *mut u32;

pub struct Router {
    buffer: Vec<Vec<u8>>,
}

impl Router {
    fn transmit_available(&self) -> bool {
        true
    }

    fn receive_available(&self) -> bool {
        self.buffer.len() > 0
    }
}

#[derive(Clone)]
pub struct RouterDriver(Arc<Mutex<Router>>);

pub struct RouterRxToken(RouterDriver);
pub struct RouterTxToken(RouterDriver);

impl<'a> phy::Device<'a> for RouterDriver {
    type RxToken = RouterRxToken;
    type TxToken = RouterTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let driver = self.0.lock();
        if driver.transmit_available() && driver.receive_available() {
            // potential racing
            Some((RouterRxToken(self.clone()), RouterTxToken(self.clone())))
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        let driver = self.0.lock();
        if driver.transmit_available() {
            Some(RouterTxToken(self.clone()))
        } else {
            None
        }
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = 1536;
        caps.max_burst_size = Some(1);
        caps
    }
}

impl phy::RxToken for RouterRxToken {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> Result<R>,
    {
        let mut router = (self.0).0.lock();
        let buffer = router.buffer.pop().unwrap();
        f(&buffer)
    }
}

impl phy::TxToken for RouterTxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = vec![0; len];
        let res = f(&mut buffer);
        debug!("out buf {}", len);

        unsafe {
            AXI_STREAM_FIFO_TDR.write_volatile(2);
            for byte in buffer {
                AXI_STREAM_FIFO_TDFD.write_volatile(byte as u32);
            }
            AXI_STREAM_FIFO_TLR.write(len as u32);
        }
        res
    }
}

pub struct RouterInterface {
    iface: Mutex<EthernetInterface<'static, 'static, 'static, RouterDriver>>,
    driver: RouterDriver,
}

impl Driver for RouterInterface {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        let mut driver = self.driver.0.lock();

        let isr = unsafe { AXI_STREAM_FIFO_ISR.read_volatile() };

        if isr > 0 {
            debug!("handle router interrupt {:b}", isr);
            unsafe {
                AXI_STREAM_FIFO_ISR.write(isr);
                let rdfo = AXI_STREAM_FIFO_RDFO.read_volatile();
                if rdfo > 0 {
                    let mut buffer = Vec::new();
                    let rlr = AXI_STREAM_FIFO_RLR.read_volatile();
                    let rdr = AXI_STREAM_FIFO_RDR.read_volatile();
                    for i in 0..rdfo {
                        buffer.push(AXI_STREAM_FIFO_RDFD.read_volatile() as u8);
                    }
                    debug!("got packet of length {}", rdfo);
                    driver.buffer.push(buffer);
                }
                drop(driver);

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
            return true;
        }
        return false;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }

    fn get_id(&self) -> String {
        format!("router")
    }

    fn get_mac(&self) -> EthernetAddress {
        unimplemented!()
    }

    fn get_ifname(&self) -> String {
        format!("router")
    }

    fn ipv4_address(&self) -> Option<Ipv4Address> {
        unimplemented!()
    }

    fn poll(&self) {
        unimplemented!()
    }
}

pub fn router_init() -> Arc<RouterInterface> {
    let ethernet_addr = EthernetAddress::from_bytes(&[2, 2, 3, 3, 0, 0]);

    let net_driver = RouterDriver(Arc::new(Mutex::new(Router { buffer: Vec::new() })));

    let ip_addrs = [
        IpCidr::new(IpAddress::v4(10, 0, 0, 1), 24),
        IpCidr::new(IpAddress::v4(10, 0, 1, 1), 24),
    ];
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let routes = Routes::new(BTreeMap::new());
    let iface = EthernetInterfaceBuilder::new(net_driver.clone())
        .ethernet_addr(ethernet_addr)
        .ip_addrs(ip_addrs)
        .neighbor_cache(neighbor_cache)
        .routes(routes)
        .finalize();

    info!("router interface up");

    let router_iface = RouterInterface {
        iface: Mutex::new(iface),
        driver: net_driver,
    };

    let driver = Arc::new(router_iface);
    DRIVERS.write().push(driver.clone());
    NET_DRIVERS.write().push(driver.clone());

    const AXI_STREAM_FIFO_IER: *mut u32 = (KERNEL_OFFSET + 0x1820_0004) as *mut u32;
    // Enable Receive Complete Interrupt
    unsafe {
        AXI_STREAM_FIFO_IER.write_volatile(1 << 26);
    }

    driver
}
