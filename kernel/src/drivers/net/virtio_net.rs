use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;

use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, Ipv4Address};
use smoltcp::Result;
use virtio_drivers::{VirtIOHeader, VirtIONet};

use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER, NET_DRIVERS};
use crate::sync::SpinNoIrqLock as Mutex;

#[derive(Clone)]
pub struct VirtIONetDriver(Arc<Mutex<VirtIONet<'static>>>);

impl Driver for VirtIONetDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        self.0.lock().ack_interrupt()
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }

    fn get_id(&self) -> String {
        format!("virtio_net")
    }

    fn get_mac(&self) -> EthernetAddress {
        EthernetAddress(self.0.lock().mac())
    }

    fn get_ifname(&self) -> String {
        format!("virtio{:?}", self.0.lock().mac())
    }

    fn ipv4_address(&self) -> Option<Ipv4Address> {
        unimplemented!()
    }

    fn poll(&self) {
        unimplemented!()
    }
}

impl phy::Device<'_> for VirtIONetDriver {
    type RxToken = VirtIONetDriver;
    type TxToken = VirtIONetDriver;

    fn receive(&mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let mut net = self.0.lock();
        if net.can_recv() {
            Some((self.clone(), self.clone()))
        } else {
            None
        }
    }

    fn transmit(&mut self) -> Option<Self::TxToken> {
        let mut net = self.0.lock();
        if net.can_send() {
            Some(self.clone())
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

impl phy::RxToken for VirtIONetDriver {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = [0u8; 2000];
        let mut driver = self.0.lock();
        let len = driver.recv(&mut buffer).expect("failed to recv packet");
        f(&mut buffer[..len])
    }
}

impl phy::TxToken for VirtIONetDriver {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut buffer = [0u8; 2000];
        let result = f(&mut buffer[..len]);
        let mut driver = self.0.lock();
        driver.send(&buffer).expect("failed to send packet");
        result
    }
}

pub fn init(header: &'static mut VirtIOHeader) {
    let net = VirtIONet::new(header).expect("failed to create net driver");
    let driver = Arc::new(VirtIONetDriver(Arc::new(Mutex::new(net))));

    DRIVERS.write().push(driver.clone());
    IRQ_MANAGER.write().register_all(driver.clone());
    NET_DRIVERS.write().push(driver);
}
