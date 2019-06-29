use alloc::string::String;
use alloc::sync::Arc;
use core::cmp::min;
use core::mem::{size_of, zeroed};
use core::slice;

use bitflags::*;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::PAGE_SIZE;
use volatile::Volatile;

use crate::sync::SpinNoIrqLock as Mutex;

use super::super::bus::virtio_mmio::*;
use super::super::{DeviceType, Driver, BLK_DRIVERS, DRIVERS, IRQ_MANAGER};
use crate::memory::phys_to_virt;

pub struct VirtIOBlk {
    interrupt_parent: u32,
    interrupt: u32,
    header: usize,
    queue: VirtIOVirtqueue,
    capacity: usize,
}

pub struct VirtIOBlkDriver(Mutex<VirtIOBlk>);

#[repr(C)]
#[derive(Debug)]
struct VirtIOBlkConfig {
    capacity: Volatile<u64>, // number of 512 sectors
}

#[repr(C)]
#[derive(Debug, Default)]
struct VirtIOBlkReadReq {
    req_type: u32,
    reserved: u32,
    sector: u64,
}

#[repr(C)]
struct VirtIOBlkWriteReq {
    req_type: u32,
    reserved: u32,
    sector: u64,
    data: [u8; VIRTIO_BLK_BLK_SIZE],
}

#[repr(C)]
struct VirtIOBlkReadResp {
    data: [u8; VIRTIO_BLK_BLK_SIZE],
    status: u8,
}

#[repr(C)]
#[derive(Debug, Default)]
struct VirtIOBlkWriteResp {
    status: u8,
}

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

const VIRTIO_BLK_BLK_SIZE: usize = 512;

bitflags! {
    struct VirtIOBlkFeature : u64 {
        const BARRIER = 1 << 0;
        const SIZE_MAX = 1 << 1;
        const SEG_MAX = 1 << 2;
        const GEOMETRY = 1 << 4;
        const RO = 1 << 5;
        const BLK_SIZE = 1 << 6;
        const SCSI = 1 << 7;
        const FLUSH = 1 << 9;
        const TOPOLOGY = 1 << 10;
        const CONFIG_WCE = 1 << 11;
        const DISCARD = 1 << 13;
        const WRITE_ZEROES = 1 << 14;
        // device independent
        const NOTIFY_ON_EMPTY = 1 << 24; // legacy
        const ANY_LAYOUT = 1 << 27; // legacy
        const RING_INDIRECT_DESC = 1 << 28;
        const RING_EVENT_IDX = 1 << 29;
        const UNUSED = 1 << 30; // legacy
        const VERSION_1 = 1 << 32; // detect legacy
        const ACCESS_PLATFORM = 1 << 33; // since virtio v1.1
        const RING_PACKED = 1 << 34; // since virtio v1.1
        const IN_ORDER = 1 << 35; // since virtio v1.1
        const ORDER_PLATFORM = 1 << 36; // since virtio v1.1
        const SR_IOV = 1 << 37; // since virtio v1.1
        const NOTIFICATION_DATA = 1 << 38; // since virtio v1.1
    }
}

impl Driver for VirtIOBlkDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        let driver = self.0.lock();

        let header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
        let interrupt = header.interrupt_status.read();
        if interrupt != 0 {
            header.interrupt_ack.write(interrupt);
            return true;
        }
        return false;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }

    fn get_id(&self) -> String {
        format!("virtio_block")
    }

    fn read_block(&self, block_id: usize, buf: &mut [u8]) -> bool {
        let mut driver = self.0.lock();
        let mut req = VirtIOBlkReadReq::default();
        req.req_type = VIRTIO_BLK_T_IN;
        req.reserved = 0;
        req.sector = block_id as u64;
        let input = [0; size_of::<VirtIOBlkReadResp>()];
        let output = unsafe {
            slice::from_raw_parts(
                &req as *const VirtIOBlkReadReq as *const u8,
                size_of::<VirtIOBlkReadReq>(),
            )
        };
        driver.queue.add_and_notify(&[&input], &[output], 0);
        driver.queue.get_block();
        let resp = unsafe { &*(&input as *const u8 as *const VirtIOBlkReadResp) };
        if resp.status == VIRTIO_BLK_S_OK {
            let len = min(buf.len(), VIRTIO_BLK_BLK_SIZE);
            buf[..len].clone_from_slice(&resp.data[..len]);
            true
        } else {
            false
        }
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) -> bool {
        let mut driver = self.0.lock();
        let mut req: VirtIOBlkWriteReq = unsafe { zeroed() };
        req.req_type = VIRTIO_BLK_T_OUT;
        req.reserved = 0;
        req.sector = block_id as u64;
        let len = min(buf.len(), VIRTIO_BLK_BLK_SIZE);
        req.data[..len].clone_from_slice(&buf[..len]);
        let input = [0; size_of::<VirtIOBlkWriteResp>()];
        let output = unsafe {
            slice::from_raw_parts(
                &req as *const VirtIOBlkWriteReq as *const u8,
                size_of::<VirtIOBlkWriteReq>(),
            )
        };
        driver.queue.add_and_notify(&[&input], &[output], 0);
        driver.queue.get_block();
        let resp = unsafe { &*(&input as *const u8 as *const VirtIOBlkWriteResp) };
        if resp.status == VIRTIO_BLK_S_OK {
            true
        } else {
            false
        }
    }
}

pub fn virtio_blk_init(node: &Node) {
    let reg = node.prop_raw("reg").unwrap();
    let paddr = reg.as_slice().read_be_u64(0).unwrap();
    let vaddr = phys_to_virt(paddr as usize);
    let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };

    header.status.write(VirtIODeviceStatus::DRIVER.bits());

    let device_features_bits = header.read_device_features();
    let device_features = VirtIOBlkFeature::from_bits_truncate(device_features_bits);
    info!("Device features {:?}", device_features);

    // negotiate these flags only
    let supported_features = VirtIOBlkFeature::empty();
    let driver_features = (device_features & supported_features).bits();
    header.write_driver_features(driver_features);

    // read configuration space
    let config = unsafe { &mut *((vaddr + VIRTIO_CONFIG_SPACE_OFFSET) as *mut VirtIOBlkConfig) };
    info!("Config: {:?}", config);
    info!(
        "Found a block device of size {}KB",
        config.capacity.read() / 2
    );

    // virtio 4.2.4 Legacy interface
    // configure two virtqueues: ingress and egress
    header.guest_page_size.write(PAGE_SIZE as u32); // one page

    let driver = VirtIOBlkDriver(Mutex::new(VirtIOBlk {
        interrupt: node.prop_u32("interrupts").unwrap(),
        interrupt_parent: node.prop_u32("interrupt-parent").unwrap(),
        header: vaddr as usize,
        queue: VirtIOVirtqueue::new(header, 0, 16),
        capacity: config.capacity.read() as usize,
    }));

    header.status.write(VirtIODeviceStatus::DRIVER_OK.bits());

    let driver = Arc::new(driver);
    DRIVERS.write().push(driver.clone());
    IRQ_MANAGER.write().register_all(driver.clone());
    BLK_DRIVERS.write().push(driver);
}
