use alloc::alloc::{GlobalAlloc, Layout};
use alloc::format;
use alloc::string::String;
use alloc::sync::Arc;
use core::mem::size_of;
use core::slice;

use bitflags::*;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::PAGE_SIZE;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::time::Instant;
use smoltcp::wire::{EthernetAddress, Ipv4Address};
use smoltcp::Result;
use volatile::{ReadOnly, Volatile};

use crate::sync::SpinNoIrqLock as Mutex;
use crate::HEAP_ALLOCATOR;

use super::super::bus::virtio_mmio::*;
use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER, NET_DRIVERS};
use crate::memory::phys_to_virt;

pub struct VirtIONet {
    interrupt_parent: u32,
    interrupt: u32,
    header: usize,
    mac: EthernetAddress,
    // 0 for receive, 1 for transmit
    queues: [VirtIOVirtqueue; 2],
}

#[derive(Clone)]
pub struct VirtIONetDriver(Arc<Mutex<VirtIONet>>);

const VIRTIO_QUEUE_RECEIVE: usize = 0;
const VIRTIO_QUEUE_TRANSMIT: usize = 1;

impl Driver for VirtIONetDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        let driver = self.0.lock();

        let header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
        let interrupt = header.interrupt_status.read();
        if interrupt != 0 {
            header.interrupt_ack.write(interrupt);
            let interrupt_status = VirtIONetworkInterruptStatus::from_bits_truncate(interrupt);
            debug!("Got interrupt {:?}", interrupt_status);

            return true;
        } else {
            return false;
        }
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }

    fn get_id(&self) -> String {
        format!("virtio_net")
    }

    fn get_mac(&self) -> EthernetAddress {
        self.0.lock().mac
    }

    fn get_ifname(&self) -> String {
        format!("virtio{}", self.0.lock().interrupt)
    }

    fn ipv4_address(&self) -> Option<Ipv4Address> {
        unimplemented!()
    }

    fn poll(&self) {
        unimplemented!()
    }
}

impl VirtIONet {
    fn transmit_available(&self) -> bool {
        self.queues[VIRTIO_QUEUE_TRANSMIT].can_add(1, 0)
    }

    fn receive_available(&self) -> bool {
        self.queues[VIRTIO_QUEUE_RECEIVE].can_get()
    }
}

pub struct VirtIONetRxToken(VirtIONetDriver);
pub struct VirtIONetTxToken(VirtIONetDriver);

impl<'a> phy::Device<'a> for VirtIONetDriver {
    type RxToken = VirtIONetRxToken;
    type TxToken = VirtIONetTxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let driver = self.0.lock();
        if driver.transmit_available() && driver.receive_available() {
            // potential racing
            Some((
                VirtIONetRxToken(self.clone()),
                VirtIONetTxToken(self.clone()),
            ))
        } else {
            None
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        let driver = self.0.lock();
        if driver.transmit_available() {
            Some(VirtIONetTxToken(self.clone()))
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

impl phy::RxToken for VirtIONetRxToken {
    fn consume<R, F>(self, _timestamp: Instant, f: F) -> Result<R>
    where
        F: FnOnce(&[u8]) -> Result<R>,
    {
        let (input, output, _, user_data) = {
            let mut driver = (self.0).0.lock();
            driver.queues[VIRTIO_QUEUE_RECEIVE].get().unwrap()
        };
        let result = f(&input[0][size_of::<VirtIONetHeader>()..]);

        let mut driver = (self.0).0.lock();
        driver.queues[VIRTIO_QUEUE_RECEIVE].add_and_notify(&input, &output, user_data);
        result
    }
}

impl phy::TxToken for VirtIONetTxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
    where
        F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let output = {
            let mut driver = (self.0).0.lock();
            if let Some((_, output, _, _)) = driver.queues[VIRTIO_QUEUE_TRANSMIT].get() {
                unsafe { slice::from_raw_parts_mut(output[0].as_ptr() as *mut u8, output[0].len()) }
            } else {
                // allocate a page for buffer
                let page = unsafe {
                    HEAP_ALLOCATOR
                        .alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
                } as usize;
                unsafe { slice::from_raw_parts_mut(page as *mut u8, PAGE_SIZE) }
            }
        };
        let output_buffer =
            &mut output[size_of::<VirtIONetHeader>()..(size_of::<VirtIONetHeader>() + len)];
        let result = f(output_buffer);

        let mut driver = (self.0).0.lock();
        assert!(driver.queues[VIRTIO_QUEUE_TRANSMIT].add_and_notify(&[], &[output], 0));
        result
    }
}

bitflags! {
    struct VirtIONetFeature : u64 {
        const CSUM = 1 << 0;
        const GUEST_CSUM = 1 << 1;
        const CTRL_GUEST_OFFLOADS = 1 << 2;
        const MTU = 1 << 3;
        const MAC = 1 << 5;
        const GSO = 1 << 6;
        const GUEST_TSO4 = 1 << 7;
        const GUEST_TSO6 = 1 << 8;
        const GUEST_ECN = 1 << 9;
        const GUEST_UFO = 1 << 10;
        const HOST_TSO4 = 1 << 11;
        const HOST_TSO6 = 1 << 12;
        const HOST_ECN = 1 << 13;
        const HOST_UFO = 1 << 14;
        const MRG_RXBUF = 1 << 15;
        const STATUS = 1 << 16;
        const CTRL_VQ = 1 << 17;
        const CTRL_RX = 1 << 18;
        const CTRL_VLAN = 1 << 19;
        const CTRL_RX_EXTRA = 1 << 20;
        const GUEST_ANNOUNCE = 1 << 21;
        const MQ = 1 << 22;
        const CTL_MAC_ADDR = 1 << 23;
        // device independent
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX = 1 << 29;
        const VERSION_1 = 1 << 32; // legacy
    }
}

bitflags! {
    struct VirtIONetworkStatus : u16 {
        const LINK_UP = 1;
        const ANNOUNCE = 2;
    }
}

bitflags! {
    struct VirtIONetworkInterruptStatus : u32 {
        const USED_RING_UPDATE = 1 << 0;
        const CONFIGURATION_CHANGE = 1 << 1;
    }
}

#[repr(C)]
#[derive(Debug)]
struct VirtIONetworkConfig {
    mac: [u8; 6],
    status: ReadOnly<u16>,
}

// virtio 5.1.6 Device Operation
#[repr(C)]
#[derive(Debug)]
struct VirtIONetHeader {
    flags: Volatile<u8>,
    gso_type: Volatile<u8>,
    hdr_len: Volatile<u16>, // cannot rely on this
    gso_size: Volatile<u16>,
    csum_start: Volatile<u16>,
    csum_offset: Volatile<u16>,
    // payload starts from here
}

pub fn virtio_net_init(node: &Node) {
    let reg = node.prop_raw("reg").unwrap();
    let paddr = reg.as_slice().read_be_u64(0).unwrap();
    let vaddr = phys_to_virt(paddr as usize);
    let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };

    header.status.write(VirtIODeviceStatus::DRIVER.bits());

    let device_features_bits = header.read_device_features();
    let device_features = VirtIONetFeature::from_bits_truncate(device_features_bits);
    debug!("Device features {:?}", device_features);

    // negotiate these flags only
    let supported_features = VirtIONetFeature::MAC | VirtIONetFeature::STATUS;
    let driver_features = (device_features & supported_features).bits();
    header.write_driver_features(driver_features);

    // read configuration space
    let config =
        unsafe { &mut *((vaddr + VIRTIO_CONFIG_SPACE_OFFSET) as *mut VirtIONetworkConfig) };
    let mac = config.mac;
    let status = VirtIONetworkStatus::from_bits_truncate(config.status.read());
    debug!("Got MAC address {:?} and status {:?}", mac, status);

    // virtio 4.2.4 Legacy interface
    // configure two virtqueues: ingress and egress
    header.guest_page_size.write(PAGE_SIZE as u32); // one page

    let queue_num = 2; // for simplicity
    let mut driver = VirtIONet {
        interrupt: node.prop_u32("interrupts").unwrap(),
        interrupt_parent: node.prop_u32("interrupt-parent").unwrap(),
        header: vaddr as usize,
        mac: EthernetAddress(mac),
        queues: [
            VirtIOVirtqueue::new(header, VIRTIO_QUEUE_RECEIVE, queue_num),
            VirtIOVirtqueue::new(header, VIRTIO_QUEUE_TRANSMIT, queue_num),
        ],
    };

    // allocate a page for buffer
    let page = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
    } as usize;
    let input = unsafe { slice::from_raw_parts(page as *const u8, PAGE_SIZE) };
    driver.queues[VIRTIO_QUEUE_RECEIVE].add_and_notify(&[input], &[], 0);

    header.status.write(VirtIODeviceStatus::DRIVER_OK.bits());

    let net_driver = Arc::new(VirtIONetDriver(Arc::new(Mutex::new(driver))));

    DRIVERS.write().push(net_driver.clone());
    IRQ_MANAGER.write().register_all(net_driver.clone());
    NET_DRIVERS.write().push(net_driver);
}
