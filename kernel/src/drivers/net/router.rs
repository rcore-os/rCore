//! rCore Router Driver

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use bitflags::*;
use smoltcp::iface::*;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::*;
use smoltcp::Result;

use crate::net::SOCKETS;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER, NET_DRIVERS, SOCKET_ACTIVITY};
use crate::memory::phys_to_virt;

const AXI_STREAM_FIFO_ISR: *mut u32 = phys_to_virt(0x64A0_0000) as *mut u32;
const AXI_STREAM_FIFO_IER: *mut u32 = phys_to_virt(0x64A0_0004) as *mut u32;
const AXI_STREAM_FIFO_TDFR: *mut u32 = phys_to_virt(0x64A0_0008) as *mut u32;
const AXI_STREAM_FIFO_TDFD: *mut u32 = phys_to_virt(0x64A0_0010) as *mut u32;
const AXI_STREAM_FIFO_TLR: *mut u32 = phys_to_virt(0x64A0_0014) as *mut u32;
const AXI_STREAM_FIFO_RDFR: *mut u32 = phys_to_virt(0x64A0_0018) as *mut u32;
const AXI_STREAM_FIFO_RDFO: *mut u32 = phys_to_virt(0x64A0_001C) as *mut u32;
const AXI_STREAM_FIFO_RDFD: *mut u32 = phys_to_virt(0x64A0_0020) as *mut u32;
const AXI_STREAM_FIFO_RLR: *mut u32 = phys_to_virt(0x64A0_0024) as *mut u32;
const AXI_STREAM_FIFO_TDR: *mut u32 = phys_to_virt(0x64A0_002C) as *mut u32;
const AXI_STREAM_FIFO_RDR: *mut u32 = phys_to_virt(0x64A0_0030) as *mut u32;

const ENABLED_PORTS: u8 = 2;

bitflags! {
    struct AXIStreamFifoInterrupt : u32 {
        const RECV_EMPTY = 1 << 19;
        const RECV_FULL = 1 << 20;
        const TRAN_EMPTY = 1 << 21;
        const TRAN_FULL = 1 << 22;
        const RECV_RESET = 1 << 23;
        const TRAN_RESET = 1 << 24;
        const TRAN_SIZE_ERR = 1 << 25;
        const RECV_COMPLETE = 1 << 26;
        const TRAN_COMPLETE = 1 << 27;
        const TRAN_PACKET_OVERRUN_ERR = 1 << 28;
        const RECV_PACKET_UNDERRUN_ERR = 1 << 29;
        const RECV_PACKET_OVERRUN_READ_ERR = 1 << 30;
        const RECV_PACKET_UNDERRUN_READ_ERR = 1 << 31;
    }
}

pub struct Router {
    buffer: [Vec<Vec<u8>>; ENABLED_PORTS as usize],
}

impl Router {
    fn transmit_available(&self) -> bool {
        true
    }

    fn receive_available(&self, port: u8) -> bool {
        self.buffer[port as usize].len() > 0
    }
}

#[derive(Clone)]
pub struct RouterDriver(Arc<Mutex<Router>>, u8);

pub struct RouterRxToken(RouterDriver);
pub struct RouterTxToken(RouterDriver);

impl<'a> phy::Device<'a> for RouterDriver {
    type RxToken = RouterRxToken;
    type TxToken = RouterTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let driver = self.0.lock();
        if driver.transmit_available() && driver.receive_available(self.1) {
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
        let buffer = router.buffer[(self.0).1 as usize].pop().unwrap();
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
        debug!(
            "out buf {} data {:x?} port {}",
            len,
            &buffer[..20],
            (self.0).1
        );

        unsafe {
            AXI_STREAM_FIFO_TDR.write_volatile(2);
            AXI_STREAM_FIFO_TDFD.write_volatile((self.0).1 as u32);
            for byte in buffer {
                AXI_STREAM_FIFO_TDFD.write_volatile(byte as u32);
            }
            AXI_STREAM_FIFO_TLR.write(((len + 1) * 4) as u32);
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
            debug!(
                "handle router interrupt {:?}",
                AXIStreamFifoInterrupt::from_bits_truncate(isr)
            );
            unsafe {
                AXI_STREAM_FIFO_ISR.write(isr);
                let rdfo = AXI_STREAM_FIFO_RDFO.read_volatile();
                if rdfo > 0 {
                    let mut buffer = Vec::new();
                    let rlr = AXI_STREAM_FIFO_RLR.read_volatile();
                    let rdr = AXI_STREAM_FIFO_RDR.read_volatile();
                    let port = AXI_STREAM_FIFO_RDFD.read_volatile();
                    for i in 1..rdfo {
                        buffer.push(AXI_STREAM_FIFO_RDFD.read_volatile() as u8);
                    }
                    debug!(
                        "got packet of length {} port {} data {:x?}",
                        rdfo,
                        port,
                        &buffer[..20]
                    );
                    driver.buffer[port as usize].push(buffer);
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

pub fn router_init() {
    unsafe {
        // reset tx fifo
        AXI_STREAM_FIFO_TDFR.write_volatile(0xA5);
        // reset rx fifo
        AXI_STREAM_FIFO_RDFR.write_volatile(0xA5);
    }

    for i in 0..ENABLED_PORTS {
        let ethernet_addr = EthernetAddress::from_bytes(&[2, 2, 3, 3, 0, i]);

        let net_driver = RouterDriver(
            Arc::new(Mutex::new(Router {
                buffer: [Vec::new(), Vec::new()],
            })),
            i,
        );

        let ip_addrs = [IpCidr::new(IpAddress::v4(10, 0, i, 1), 24)];
        let neighbor_cache = NeighborCache::new(BTreeMap::new());
        let routes = Routes::new(BTreeMap::new());
        let iface = EthernetInterfaceBuilder::new(net_driver.clone())
            .ethernet_addr(ethernet_addr)
            .ip_addrs(ip_addrs)
            .neighbor_cache(neighbor_cache)
            .routes(routes)
            .finalize();

        info!("router interface up #{}", i);

        let router_iface = RouterInterface {
            iface: Mutex::new(iface),
            driver: net_driver,
        };

        let driver = Arc::new(router_iface);
        DRIVERS.write().push(driver.clone());
        IRQ_MANAGER.write().register_all(driver.clone());
        NET_DRIVERS.write().push(driver.clone());
    }

    // Enable Receive Complete Interrupt
    unsafe {
        AXI_STREAM_FIFO_IER.write_volatile(1 << 26);
    }
}
