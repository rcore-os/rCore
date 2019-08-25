use alloc::alloc::{GlobalAlloc, Layout};
use alloc::string::String;
use alloc::sync::Arc;
use core::slice;

use bitflags::*;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::PAGE_SIZE;
use volatile::{ReadOnly, Volatile, WriteOnly};

use crate::arch::cpu;
use crate::memory::virt_to_phys;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::HEAP_ALLOCATOR;

use super::super::bus::virtio_mmio::*;
use super::super::{DeviceType, Driver, DRIVERS, IRQ_MANAGER};
use super::test::mandelbrot;
use crate::memory::phys_to_virt;

const VIRTIO_GPU_EVENT_DISPLAY: u32 = 1 << 0;

struct VirtIOGpu {
    interrupt_parent: u32,
    interrupt: u32,
    header: &'static mut VirtIOHeader,
    queue_buffer: [usize; 2],
    frame_buffer: usize,
    rect: VirtIOGpuRect,
    queues: [VirtIOVirtqueue; 2],
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuConfig {
    events_read: ReadOnly<u32>,
    events_clear: WriteOnly<u32>,
    num_scanouts: Volatile<u32>,
}

bitflags! {
    struct VirtIOGpuFeature : u64 {
        const VIRGL = 1 << 0;
        const EDID = 1 << 1;
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

const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x100;
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x101;
const VIRTIO_GPU_CMD_RESOURCE_UNREF: u32 = 0x102;
const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x103;
const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x104;
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x105;
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x106;
const VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING: u32 = 0x107;
const VIRTIO_GPU_CMD_GET_CAPSET_INFO: u32 = 0x108;
const VIRTIO_GPU_CMD_GET_CAPSET: u32 = 0x109;
const VIRTIO_GPU_CMD_GET_EDID: u32 = 0x10a;

const VIRTIO_GPU_CMD_UPDATE_CURSOR: u32 = 0x300;
const VIRTIO_GPU_CMD_MOVE_CURSOR: u32 = 0x301;

const VIRTIO_GPU_RESP_OK_NODATA: u32 = 0x1100;
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO: u32 = 0x1101;
const VIRTIO_GPU_RESP_OK_CAPSET_INFO: u32 = 0x1102;
const VIRTIO_GPU_RESP_OK_CAPSET: u32 = 0x1103;
const VIRTIO_GPU_RESP_OK_EDID: u32 = 0x1104;

const VIRTIO_GPU_RESP_ERR_UNSPEC: u32 = 0x1200;
const VIRTIO_GPU_RESP_ERR_OUT_OF_MEMORY: u32 = 0x1201;
const VIRTIO_GPU_RESP_ERR_INVALID_SCANOUT_ID: u32 = 0x1202;

const VIRTIO_GPU_FLAG_FENCE: u32 = 1 << 0;

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuCtrlHdr {
    hdr_type: u32,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    padding: u32,
}

impl VirtIOGpuCtrlHdr {
    fn with_type(hdr_type: u32) -> VirtIOGpuCtrlHdr {
        VirtIOGpuCtrlHdr {
            hdr_type,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            padding: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct VirtIOGpuRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuRespDisplayInfo {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    enabled: u32,
    flags: u32,
}

const VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM: u32 = 1;

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuResourceCreate2D {
    header: VirtIOGpuCtrlHdr,
    resource_id: u32,
    format: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuResourceAttachBacking {
    header: VirtIOGpuCtrlHdr,
    resource_id: u32,
    nr_entries: u32, // always 1
    addr: u64,
    length: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuSetScanout {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    scanout_id: u32,
    resource_id: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuTransferToHost2D {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    offset: u64,
    resource_id: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Debug)]
struct VirtIOGpuResourceFlush {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    resource_id: u32,
    padding: u32,
}

const VIRTIO_QUEUE_TRANSMIT: usize = 0;
const VIRTIO_QUEUE_CURSOR: usize = 1;

const VIRTIO_BUFFER_TRANSMIT: usize = 0;
const VIRTIO_BUFFER_RECEIVE: usize = 1;

const VIRTIO_GPU_RESOURCE_ID: u32 = 0xbabe;

pub struct VirtIOGpuDriver(Mutex<VirtIOGpu>);

impl Driver for VirtIOGpuDriver {
    fn try_handle_interrupt(&self, _irq: Option<u32>) -> bool {
        // for simplicity
        if cpu::id() > 0 {
            return false;
        }

        let mut driver = self.0.lock();

        let interrupt = driver.header.interrupt_status.read();
        if interrupt != 0 {
            driver.header.interrupt_ack.write(interrupt);
            debug!("Got interrupt {:?}", interrupt);
            return true;
        }
        return false;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Gpu
    }

    fn get_id(&self) -> String {
        format!("virtio_gpu")
    }
}

fn request(driver: &mut VirtIOGpu) {
    let input = unsafe {
        slice::from_raw_parts(
            driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *const u8,
            PAGE_SIZE,
        )
    };
    let output = unsafe {
        slice::from_raw_parts(
            driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *const u8,
            PAGE_SIZE,
        )
    };
    driver.queues[VIRTIO_QUEUE_TRANSMIT].add_and_notify(&[input], &[output], 0);
}

fn setup_framebuffer(driver: &mut VirtIOGpu) {
    // get display info
    let request_get_display_info =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *mut VirtIOGpuCtrlHdr) };
    *request_get_display_info = VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_GET_DISPLAY_INFO);
    request(driver);
    driver.queues[VIRTIO_QUEUE_TRANSMIT].get_block();
    let response_get_display_info = unsafe {
        &mut *(driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *mut VirtIOGpuRespDisplayInfo)
    };
    info!("response: {:?}", response_get_display_info);
    driver.rect = response_get_display_info.rect;

    // create resource 2d
    let request_resource_create_2d = unsafe {
        &mut *(driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *mut VirtIOGpuResourceCreate2D)
    };
    *request_resource_create_2d = VirtIOGpuResourceCreate2D {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_RESOURCE_CREATE_2D),
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        format: VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
        width: response_get_display_info.rect.width,
        height: response_get_display_info.rect.height,
    };
    request(driver);
    driver.queues[VIRTIO_QUEUE_TRANSMIT].get_block();
    let response_resource_create_2d =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_resource_create_2d);

    // alloc continuous pages for the frame buffer
    let size = response_get_display_info.rect.width * response_get_display_info.rect.height * 4;
    let frame_buffer = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(size as usize, PAGE_SIZE).unwrap())
    } as usize;
    // test framebuffer
//    mandelbrot(
//        driver.rect.width,
//        driver.rect.height,
//        frame_buffer as *mut u32,
//    );
    driver.frame_buffer = frame_buffer;
    let request_resource_attach_backing = unsafe {
        &mut *(driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *mut VirtIOGpuResourceAttachBacking)
    };
    *request_resource_attach_backing = VirtIOGpuResourceAttachBacking {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING),
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        nr_entries: 1,
        addr: virt_to_phys(frame_buffer) as u64,
        length: size,
        padding: 0,
    };
    request(driver);
    driver.queues[VIRTIO_QUEUE_TRANSMIT].get_block();
    let response_resource_attach_backing =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_resource_attach_backing);

    // map frame buffer to screen
    let request_set_scanout =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *mut VirtIOGpuSetScanout) };
    *request_set_scanout = VirtIOGpuSetScanout {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_SET_SCANOUT),
        rect: response_get_display_info.rect,
        scanout_id: 0,
        resource_id: VIRTIO_GPU_RESOURCE_ID,
    };
    request(driver);
    driver.queues[VIRTIO_QUEUE_TRANSMIT].get_block();
    let response_set_scanout =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_set_scanout);

    flush_frame_buffer_to_screen(driver);
}

fn flush_frame_buffer_to_screen(driver: &mut VirtIOGpu) {
    // copy data from guest to host
    let request_transfer_to_host_2d = unsafe {
        &mut *(driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *mut VirtIOGpuTransferToHost2D)
    };
    *request_transfer_to_host_2d = VirtIOGpuTransferToHost2D {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D),
        rect: driver.rect,
        offset: 0,
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        padding: 0,
    };
    request(driver);
    driver.queues[VIRTIO_QUEUE_TRANSMIT].get_block();
    let response_transfer_to_host_2d =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_transfer_to_host_2d);

    // flush data to screen
    let request_resource_flush = unsafe {
        &mut *(driver.queue_buffer[VIRTIO_BUFFER_TRANSMIT] as *mut VirtIOGpuResourceFlush)
    };
    *request_resource_flush = VirtIOGpuResourceFlush {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_RESOURCE_FLUSH),
        rect: driver.rect,
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        padding: 0,
    };
    request(driver);
    driver.queues[VIRTIO_QUEUE_TRANSMIT].get_block();
    let response_resource_flush =
        unsafe { &mut *(driver.queue_buffer[VIRTIO_BUFFER_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_resource_flush);
}

pub fn virtio_gpu_init(node: &Node) {
    let reg = node.prop_raw("reg").unwrap();
    let paddr = reg.as_slice().read_be_u64(0).unwrap();
    let vaddr = phys_to_virt(paddr as usize);
    let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };

    header.status.write(VirtIODeviceStatus::DRIVER.bits());

    let device_features_bits = header.read_device_features();
    let device_features = VirtIOGpuFeature::from_bits_truncate(device_features_bits);
    info!("Device features {:?}", device_features);

    // negotiate these flags only
    let supported_features = VirtIOGpuFeature::empty();
    let driver_features = (device_features & supported_features).bits();
    header.write_driver_features(driver_features);

    // read configuration space
    let config = unsafe { &mut *((vaddr + VIRTIO_CONFIG_SPACE_OFFSET) as *mut VirtIOGpuConfig) };
    info!("Config: {:?}", config);

    // virtio 4.2.4 Legacy interface
    // configure two virtqueues: ingress and egress
    header.guest_page_size.write(PAGE_SIZE as u32); // one page

    let queue_num = 2;
    let queues = [
        VirtIOVirtqueue::new(header, VIRTIO_QUEUE_TRANSMIT, queue_num),
        VirtIOVirtqueue::new(header, VIRTIO_QUEUE_CURSOR, queue_num),
    ];
    let mut driver = VirtIOGpu {
        interrupt: node.prop_u32("interrupts").unwrap(),
        interrupt_parent: node.prop_u32("interrupt-parent").unwrap(),
        header,
        queue_buffer: [0, 0],
        frame_buffer: 0,
        rect: VirtIOGpuRect::default(),
        queues,
    };

    for buffer in 0..2 {
        // allocate a page for each buffer
        let page = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
        } as usize;
        driver.queue_buffer[buffer as usize] = page;
        debug!("buffer {} using page address {:#X}", buffer, page as usize);
    }

    driver
        .header
        .status
        .write(VirtIODeviceStatus::DRIVER_OK.bits());

    setup_framebuffer(&mut driver);

    let driver = Arc::new(VirtIOGpuDriver(Mutex::new(driver)));
    IRQ_MANAGER.write().register_all(driver.clone());
    DRIVERS.write().push(driver);
}
