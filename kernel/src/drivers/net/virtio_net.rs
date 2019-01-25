use alloc::alloc::{GlobalAlloc, Layout};
use alloc::format;
use alloc::prelude::*;
use alloc::sync::Arc;
use core::mem::size_of;
use core::slice;
use core::sync::atomic::{fence, Ordering};

use bitflags::*;
use device_tree::Node;
use device_tree::util::SliceRead;
use log::*;
use rcore_memory::PAGE_SIZE;
use rcore_memory::paging::PageTable;
use smoltcp::phy::{self, DeviceCapabilities};
use smoltcp::Result;
use smoltcp::time::Instant;
use smoltcp::wire::EthernetAddress;
use volatile::{ReadOnly, Volatile};

use crate::arch::cpu;
use crate::HEAP_ALLOCATOR;
use crate::memory::active_table;
use crate::sync::SpinNoIrqLock as Mutex;

use super::super::{DeviceType, Driver, DRIVERS, NET_DRIVERS, NetDriver};
use super::super::bus::virtio_mmio::*;

pub struct VirtIONet {
    interrupt_parent: u32,
    interrupt: u32,
    header: usize,
    mac: EthernetAddress,
    queue_num: u32,
    // 0 for receive, 1 for transmit
    queue_address: [usize; 2],
    queue_page: [usize; 2],
    last_used_idx: [u16; 2],
}

#[derive(Clone)]
pub struct VirtIONetDriver(Arc<Mutex<VirtIONet>>);

const VIRTIO_QUEUE_RECEIVE: usize = 0;
const VIRTIO_QUEUE_TRANSMIT: usize = 1;

impl Driver for VirtIONetDriver {
    fn try_handle_interrupt(&mut self) -> bool {
        // for simplicity
        if cpu::id() > 0 {
            return false
        }

        let mut driver = self.0.lock();

        // ensure header page is mapped
        active_table().map_if_not_exists(driver.header as usize, driver.header as usize);

        let mut header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
        let interrupt = header.interrupt_status.read();
        if interrupt != 0 {
            header.interrupt_ack.write(interrupt);
            let interrupt_status = VirtIONetworkInterruptStatus::from_bits_truncate(interrupt);
            debug!("Got interrupt {:?}", interrupt_status);
            if interrupt_status.contains(VirtIONetworkInterruptStatus::USED_RING_UPDATE) {
                // need to change when queue_num is larger than one
                let queue = VIRTIO_QUEUE_TRANSMIT;
                let used_ring_offset = virtqueue_used_elem_offset(driver.queue_num as usize, PAGE_SIZE);
                let mut used_ring = unsafe { 
                    &mut *((driver.queue_address[queue] + used_ring_offset) as *mut VirtIOVirtqueueUsedRing) 
                };
                if driver.last_used_idx[queue] < used_ring.idx.read() {
                    assert_eq!(driver.last_used_idx[queue], used_ring.idx.read() - 1);
                    info!("Processing queue {} from {} to {}", queue, driver.last_used_idx[queue], used_ring.idx.read());
                    driver.last_used_idx[queue] = used_ring.idx.read();
                }
            } else if interrupt_status.contains(VirtIONetworkInterruptStatus::CONFIGURATION_CHANGE) {
                // TODO: update mac and status
                unimplemented!("virtio-net configuration change not implemented");
            }

            return true;
        } else {
            return false;
        }
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Net
    }
}

impl VirtIONet {
    fn transmit_available(&self) -> bool {
        let used_ring_offset = virtqueue_used_elem_offset(self.queue_num as usize, PAGE_SIZE);
        let mut used_ring = unsafe { 
            &mut *((self.queue_address[VIRTIO_QUEUE_TRANSMIT] + used_ring_offset) as *mut VirtIOVirtqueueUsedRing) 
        };
        let result = self.last_used_idx[VIRTIO_QUEUE_TRANSMIT] == used_ring.idx.read();
        result
    }


    fn receive_available(&self) -> bool {
        let used_ring_offset = virtqueue_used_elem_offset(self.queue_num as usize, PAGE_SIZE);
        let mut used_ring = unsafe { 
            &mut *((self.queue_address[VIRTIO_QUEUE_RECEIVE] + used_ring_offset) as *mut VirtIOVirtqueueUsedRing) 
        };
        let result = self.last_used_idx[VIRTIO_QUEUE_RECEIVE] < used_ring.idx.read();
        result
    }
}

impl NetDriver for VirtIONetDriver {
    fn get_mac(&self) -> EthernetAddress {
        self.0.lock().mac
    }

    fn get_ifname(&self) -> String {
        format!("virtio{}", self.0.lock().interrupt)
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
            // ugly borrow rules bypass
            Some((VirtIONetRxToken(self.clone()),
                VirtIONetTxToken(self.clone())))
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
    fn consume<R, F>(self, timestamp: Instant, f: F) -> Result<R>
        where F: FnOnce(&[u8]) -> Result<R>
    {
        let buffer = {
            let mut driver = (self.0).0.lock();

            // ensure header page is mapped
            active_table().map_if_not_exists(driver.header as usize, driver.header as usize);

            let mut header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
            let used_ring_offset = virtqueue_used_elem_offset(driver.queue_num as usize, PAGE_SIZE);
            let mut used_ring = unsafe { 
                &mut *((driver.queue_address[VIRTIO_QUEUE_RECEIVE] + used_ring_offset) as *mut VirtIOVirtqueueUsedRing) 
            };
            assert!(driver.last_used_idx[VIRTIO_QUEUE_RECEIVE] == used_ring.idx.read() - 1);
            driver.last_used_idx[VIRTIO_QUEUE_RECEIVE] = used_ring.idx.read();
            let mut payload = unsafe { slice::from_raw_parts_mut((driver.queue_page[VIRTIO_QUEUE_RECEIVE] + size_of::<VirtIONetHeader>()) as *mut u8, PAGE_SIZE - 10)};
            let buffer = payload.to_vec();
            for i in 0..(PAGE_SIZE - size_of::<VirtIONetHeader>()) {
                payload[i] = 0;
            }

            let mut ring = unsafe { 
                &mut *((driver.queue_address[VIRTIO_QUEUE_RECEIVE] + size_of::<VirtIOVirtqueueDesc>() * driver.queue_num as usize) as *mut VirtIOVirtqueueAvailableRing) 
            };
            ring.idx.write(ring.idx.read() + 1);
            header.queue_notify.write(VIRTIO_QUEUE_RECEIVE as u32);
            buffer
        };
        f(&buffer)
    }
}

impl phy::TxToken for VirtIONetTxToken {
    fn consume<R, F>(self, _timestamp: Instant, len: usize, f: F) -> Result<R>
        where F: FnOnce(&mut [u8]) -> Result<R>,
    {
        let mut driver = (self.0).0.lock();

        // ensure header page is mapped
        active_table().map_if_not_exists(driver.header as usize, driver.header as usize);

        let mut header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
        let payload_target = unsafe { slice::from_raw_parts_mut((driver.queue_page[VIRTIO_QUEUE_TRANSMIT] + size_of::<VirtIONetHeader>()) as *mut u8, len)};
        let result = f(payload_target);
        let mut net_header = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIONetHeader) };

        let mut header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
        let mut ring = unsafe { 
            &mut *((driver.queue_address[VIRTIO_QUEUE_TRANSMIT] + size_of::<VirtIOVirtqueueDesc>() * driver.queue_num as usize) as *mut VirtIOVirtqueueAvailableRing) 
        };

        // re-add buffer to desc
        let mut desc = unsafe { &mut *(driver.queue_address[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOVirtqueueDesc) };
        desc.addr.write(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as u64);
        desc.len.write((len + size_of::<VirtIONetHeader>()) as u32);

        // memory barrier
        fence(Ordering::SeqCst);
        
        // add desc to available ring
        ring.idx.write(ring.idx.read() + 1);
        header.queue_notify.write(VIRTIO_QUEUE_TRANSMIT as u32);
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
    status: ReadOnly<u16>
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
    let from = reg.as_slice().read_be_u64(0).unwrap();
    let mut header = unsafe { &mut *(from as *mut VirtIOHeader) };

    header.status.write(VirtIODeviceStatus::DRIVER.bits());

    let mut device_features_bits: u64;
    header.device_features_sel.write(0); // device features [0, 32)
    device_features_bits = header.device_features.read().into();
    header.device_features_sel.write(1); // device features [32, 64)
    device_features_bits = device_features_bits + ((header.device_features.read() as u64) << 32);
    let device_features = VirtIONetFeature::from_bits_truncate(device_features_bits);
    debug!("Device features {:?}", device_features);

    // negotiate these flags only
    let supported_features = VirtIONetFeature::MAC | VirtIONetFeature::STATUS;
    let driver_features = (device_features & supported_features).bits();
    header.driver_features_sel.write(0); // driver features [0, 32)
    header.driver_features.write((driver_features & 0xFFFFFFFF) as u32);
    header.driver_features_sel.write(1); // driver features [32, 64)
    header.driver_features.write(((driver_features & 0xFFFFFFFF00000000) >> 32) as u32);

    // read configuration space
    let mut mac: [u8; 6];
    let mut status: VirtIONetworkStatus;
    let mut config = unsafe { &mut *((from + 0x100) as *mut VirtIONetworkConfig) };
    mac = config.mac;
    status = VirtIONetworkStatus::from_bits_truncate(config.status.read());
    debug!("Got MAC address {:?} and status {:?}", mac, status);

    // virtio 4.2.4 Legacy interface
    // configure two virtqueues: ingress and egress
    header.guest_page_size.write(PAGE_SIZE as u32); // one page

    let queue_num = 1; // for simplicity
    let mut driver = VirtIONet {
        interrupt: node.prop_u32("interrupts").unwrap(),
        interrupt_parent: node.prop_u32("interrupt-parent").unwrap(),
        header: from as usize,
        mac: EthernetAddress(mac),
        queue_num: queue_num,
        queue_address: [0, 0],
        queue_page: [0, 0],
        last_used_idx: [0, 0],
    };

    // 0 for receive, 1 for transmit
    for queue in 0..2 {
        header.queue_sel.write(queue as u32);
        assert_eq!(header.queue_pfn.read(), 0); // not in use
        let queue_num_max = header.queue_num_max.read();
        assert!(queue_num_max >= queue_num); // queue available
        let size = virtqueue_size(queue_num as usize, PAGE_SIZE);
        assert!(size % PAGE_SIZE == 0);
        // alloc continuous pages
        let address = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(size, PAGE_SIZE).unwrap())
        } as usize;
        driver.queue_address[queue] = address;
        debug!("queue {} using page address {:#X} with size {}", queue, address as usize, size);

        header.queue_num.write(queue_num);
        header.queue_align.write(PAGE_SIZE as u32);
        header.queue_pfn.write((address as u32) >> 12);

        // allocate a page for buffer
        let page = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
        } as usize;
        driver.queue_page[queue] = page;

        // fill first desc
        let mut desc = unsafe { &mut *(address as *mut VirtIOVirtqueueDesc) };
        desc.addr.write(page as u64);
        desc.len.write(PAGE_SIZE as u32);
        if queue == VIRTIO_QUEUE_RECEIVE {
            // device writable
            desc.flags.write(VirtIOVirtqueueFlag::WRITE.bits());
        } else if queue == VIRTIO_QUEUE_TRANSMIT {
            // driver readable
            desc.flags.write(0);
        }
        // memory barrier
        fence(Ordering::SeqCst);


        if queue == VIRTIO_QUEUE_RECEIVE {
            // add the desc to the ring
            let mut ring = unsafe { 
                &mut *((address + size_of::<VirtIOVirtqueueDesc>() * queue_num as usize) as *mut VirtIOVirtqueueAvailableRing) 
            };
            ring.ring[0].write(0);
            // wait for first packet
            ring.idx.write(ring.idx.read() + 1);
        }

        // notify device about the new buffer
        header.queue_notify.write(queue as u32);
        debug!("queue {} using page address {:#X}", queue, page);
    }

    header.status.write(VirtIODeviceStatus::DRIVER_OK.bits());

    let mut net_driver = VirtIONetDriver(Arc::new(Mutex::new(driver)));

    DRIVERS.lock().push(Box::new(net_driver.clone()));
    NET_DRIVERS.lock().push(Box::new(net_driver));
}