use alloc::alloc::{GlobalAlloc, Layout};
use alloc::prelude::*;
use core::mem::size_of;
use core::slice;

use bitflags::*;
use device_tree::Node;
use device_tree::util::SliceRead;
use log::*;
use rcore_memory::PAGE_SIZE;
use rcore_memory::paging::PageTable;
use volatile::{ReadOnly, Volatile, WriteOnly};

use crate::arch::cpu;
use crate::HEAP_ALLOCATOR;
use crate::memory::active_table;

use super::super::{DeviceType, Driver, DRIVERS};
use super::super::bus::virtio_mmio::*;

const VIRTIO_GPU_EVENT_DISPLAY : u32 = 1 << 0;

struct VirtIOGpu {
    interrupt_parent: u32,
    interrupt: u32,
    header: usize,
    // 0 for transmit, 1 for cursor
    queue_num: u32,
    queue_address: usize,
    queue_page: [usize; 2],
    last_used_idx: u16,
    frame_buffer: usize,
    rect: VirtIOGpuRect
}

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuConfig {
    events_read: ReadOnly<u32>,
    events_clear: WriteOnly<u32>,
    num_scanouts: Volatile<u32>
}

bitflags! {
    struct VirtIOGpuFeature : u64 {
        const VIRGL = 1 << 0;
        const EDID = 1 << 1;
        // device independent
        const NOFIFY_ON_EMPTY = 1 << 24; // legacy
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

const VIRTIO_GPU_CMD_GET_DISPLAY_INFO : u32 = 0x100;
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D : u32 = 0x101;
const VIRTIO_GPU_CMD_RESOURCE_UNREF : u32 = 0x102;
const VIRTIO_GPU_CMD_SET_SCANOUT : u32 = 0x103;
const VIRTIO_GPU_CMD_RESOURCE_FLUSH : u32 = 0x104;
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D : u32 = 0x105;
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING : u32 = 0x106;
const VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING : u32 = 0x107;
const VIRTIO_GPU_CMD_GET_CAPSET_INFO : u32 = 0x108;
const VIRTIO_GPU_CMD_GET_CAPSET : u32 = 0x109;
const VIRTIO_GPU_CMD_GET_EDID : u32 = 0x10a;

const VIRTIO_GPU_CMD_UPDATE_CURSOR : u32 = 0x300;
const VIRTIO_GPU_CMD_MOVE_CURSOR : u32 = 0x301;

const VIRTIO_GPU_RESP_OK_NODATA : u32 = 0x1100;
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO : u32 = 0x1101;
const VIRTIO_GPU_RESP_OK_CAPSET_INFO : u32 = 0x1102;
const VIRTIO_GPU_RESP_OK_CAPSET : u32 = 0x1103;
const VIRTIO_GPU_RESP_OK_EDID : u32 = 0x1104;

const VIRTIO_GPU_RESP_ERR_UNSPEC : u32 = 0x1200;
const VIRTIO_GPU_RESP_ERR_OUT_OF_MEMORY : u32 = 0x1201;
const VIRTIO_GPU_RESP_ERR_INVALID_SCANOUT_ID : u32 = 0x1202;

const VIRTIO_GPU_FLAG_FENCE : u32 = 1 << 0;

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuCtrlHdr {
    hdr_type: u32,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    padding: u32
}

impl VirtIOGpuCtrlHdr {
    fn with_type(hdr_type: u32) -> VirtIOGpuCtrlHdr {
        VirtIOGpuCtrlHdr {
            hdr_type,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            padding: 0
        }
    }
}

#[repr(packed)]
#[derive(Debug, Copy, Clone, Default)]
struct VirtIOGpuRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32
}

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuRespDisplayInfo {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    enabled: u32,
    flags: u32
}

const VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM: u32 = 1;

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuResourceCreate2D {
    header: VirtIOGpuCtrlHdr,
    resource_id: u32,
    format: u32,
    width: u32,
    height: u32
}

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuResourceAttachBacking {
    header: VirtIOGpuCtrlHdr,
    resource_id: u32,
    nr_entries: u32, // always 1
    addr: u64,
    length: u32,
    padding: u32
}

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuSetScanout {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    scanout_id: u32,
    resource_id: u32
}

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuTransferToHost2D {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    offset: u64,
    resource_id: u32,
    padding: u32
}

#[repr(packed)]
#[derive(Debug)]
struct VirtIOGpuResourceFlush {
    header: VirtIOGpuCtrlHdr,
    rect: VirtIOGpuRect,
    resource_id: u32,
    padding: u32
}

const VIRTIO_QUEUE_TRANSMIT: usize = 0;
const VIRTIO_QUEUE_RECEIVE: usize = 1;

const VIRTIO_GPU_RESOURCE_ID: u32 = 0xbabe;

impl Driver for VirtIOGpu {
    fn try_handle_interrupt(&mut self) -> bool {
        // for simplicity
        if cpu::id() > 0 {
            return false
        }

        // ensure header page is mapped
        active_table().map_if_not_exists(self.header as usize, self.header as usize);

        let mut header = unsafe { &mut *(self.header as *mut VirtIOHeader) };
        let interrupt = header.interrupt_status.read();
        if interrupt != 0 {
            header.interrupt_ack.write(interrupt);
            debug!("Got interrupt {:?}", interrupt);
            let response = unsafe { &mut *(self.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
            debug!("response in interrupt: {:?}", response);
            return true;
        }
        return false;
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Gpu
    }
}

fn setup_rings(driver: &mut VirtIOGpu) {
    let mut ring = unsafe { 
        &mut *((driver.queue_address + size_of::<VirtIOVirtqueueDesc>() * driver.queue_num as usize) as *mut VirtIOVirtqueueAvailableRing) 
    };

    // re-add two buffers to desc
    // chaining read buffer and write buffer into one desc
    for buffer in 0..2 {
        let mut desc = unsafe { &mut *(driver.queue_address as *mut VirtIOVirtqueueDesc).add(buffer) };
        desc.addr.write(driver.queue_page[buffer] as u64);
        desc.len.write(PAGE_SIZE as u32);
        if buffer == VIRTIO_QUEUE_TRANSMIT {
            // device readable
            desc.flags.write(VirtIOVirtqueueFlag::NEXT.bits());
            desc.next.write(1);
        } else {
            // device writable
            desc.flags.write(VirtIOVirtqueueFlag::WRITE.bits());
        }
        ring.ring[buffer].write(0);
    }
}

fn notify_device(driver: &mut VirtIOGpu) {
    let mut header = unsafe { &mut *(driver.header as *mut VirtIOHeader) };
    let mut ring = unsafe { 
        &mut *((driver.queue_address + size_of::<VirtIOVirtqueueDesc>() * driver.queue_num as usize) as *mut VirtIOVirtqueueAvailableRing) 
    };
    ring.idx.write(ring.idx.read() + 1);
    header.queue_notify.write(0);
}

fn setup_framebuffer(driver: &mut VirtIOGpu) {
    // get display info
    setup_rings(driver);
    let mut request_get_display_info = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOGpuCtrlHdr) };
    *request_get_display_info = VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_GET_DISPLAY_INFO);
    notify_device(driver);
    let response_get_display_info = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuRespDisplayInfo) };
    info!("response: {:?}", response_get_display_info);
    driver.rect = response_get_display_info.rect;

    // create resource 2d
    setup_rings(driver);
    let mut request_resource_create_2d = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOGpuResourceCreate2D) };
    *request_resource_create_2d = VirtIOGpuResourceCreate2D {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_RESOURCE_CREATE_2D),
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        format: VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM,
        width: response_get_display_info.rect.width,
        height: response_get_display_info.rect.height
    };
    notify_device(driver);
    let response_resource_create_2d = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_resource_create_2d);

    // alloc continuous pages for the frame buffer
    let size = response_get_display_info.rect.width * response_get_display_info.rect.height * 4;
    let frame_buffer = unsafe {
        HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(size as usize, PAGE_SIZE).unwrap())
    } as usize;
    mandelbrot(driver.rect.width, driver.rect.height, frame_buffer as *mut u32);
    driver.frame_buffer = frame_buffer;
    setup_rings(driver);
    let mut request_resource_attach_backing = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOGpuResourceAttachBacking) };
    *request_resource_attach_backing = VirtIOGpuResourceAttachBacking {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING),
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        nr_entries: 1,
        addr: frame_buffer as u64,
        length: size,
        padding: 0
    };
    notify_device(driver);
    let response_resource_attach_backing = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    debug!("response: {:?}", response_resource_attach_backing);

    // map frame buffer to screen
    setup_rings(driver);
    let mut request_set_scanout = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOGpuSetScanout) };
    *request_set_scanout = VirtIOGpuSetScanout {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_SET_SCANOUT),
        rect: response_get_display_info.rect,
        scanout_id: 0,
        resource_id: VIRTIO_GPU_RESOURCE_ID
    };
    notify_device(driver);
    let response_set_scanout = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_set_scanout);

    flush_frame_buffer_to_screen(driver);
}

// from Wikipedia
fn hsv_to_rgb(h: u32, s: f32, v: f32) -> (f32, f32, f32) {
    let hi = (h / 60) % 6;
    let f = (h % 60) as f32 / 60.0;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    match hi {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        5 => (v, p, q),
        _ => unreachable!()
    }
}

fn mandelbrot(width: u32, height: u32, frame_buffer: *mut u32) {
    let size = width * height * 4;
    let frame_buffer_data = unsafe {
        slice::from_raw_parts_mut(frame_buffer as *mut u32, (size / 4) as usize)
    };
    for x in 0..width {
        for y in 0..height {
            let index = y * width + x;
            let scale = 5e-3;
            let xx = (x as f32 - width as f32 / 2.0) * scale;
            let yy = (y as f32 - height as f32 / 2.0) * scale;
            let mut re = xx as f32;
            let mut im = yy as f32;
            let mut iter: u32 = 0;
            loop {
                iter = iter + 1;
                let new_re = re * re - im * im + xx as f32;
                let new_im = re * im * 2.0 + yy as f32;
                if new_re * new_re + new_im * new_im > 1e3 {
                    break;
                }
                re = new_re;
                im = new_im;

                if iter == 60 {
                    break;
                }
            }
            iter = iter * 6;
            let (r, g, b) = hsv_to_rgb(iter, 1.0, 0.5);
            let rr = (r * 256.0) as u32;
            let gg = (g * 256.0) as u32;
            let bb = (b * 256.0) as u32;
            let color = (bb << 16) | (gg << 8) | rr;
            frame_buffer_data[index as usize] = color;
        }
        println!("working on x {}/{}", x, width);
    }
}

fn flush_frame_buffer_to_screen(driver: &mut VirtIOGpu) {
    // copy data from guest to host
    setup_rings(driver);
    let mut request_transfer_to_host_2d = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOGpuTransferToHost2D) };
    *request_transfer_to_host_2d = VirtIOGpuTransferToHost2D {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D),
        rect: driver.rect,
        offset: 0,
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        padding: 0
    };
    notify_device(driver);
    let response_transfer_to_host_2d = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_transfer_to_host_2d);

    // flush data to screen
    setup_rings(driver);
    let mut request_resource_flush = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_TRANSMIT] as *mut VirtIOGpuResourceFlush) };
    *request_resource_flush = VirtIOGpuResourceFlush {
        header: VirtIOGpuCtrlHdr::with_type(VIRTIO_GPU_CMD_RESOURCE_FLUSH),
        rect: driver.rect,
        resource_id: VIRTIO_GPU_RESOURCE_ID,
        padding: 0
    };
    notify_device(driver);
    let response_resource_flush = unsafe { &mut *(driver.queue_page[VIRTIO_QUEUE_RECEIVE] as *mut VirtIOGpuCtrlHdr) };
    info!("response: {:?}", response_resource_flush);
}

pub fn virtio_gpu_init(node: &Node) {
    let reg = node.prop_raw("reg").unwrap();
    let from = reg.as_slice().read_be_u64(0).unwrap();
    let mut header = unsafe { &mut *(from as *mut VirtIOHeader) };

    header.status.write(VirtIODeviceStatus::DRIVER.bits());

    let mut device_features_bits: u64;
    header.device_features_sel.write(0); // device features [0, 32)
    device_features_bits = header.device_features.read().into();
    header.device_features_sel.write(1); // device features [32, 64)
    device_features_bits = device_features_bits + ((header.device_features.read() as u64) << 32);
    let device_features = VirtIOGpuFeature::from_bits_truncate(device_features_bits);
    info!("Device features {:?}", device_features);

    // negotiate these flags only
    let supported_features = VirtIOGpuFeature::empty();
    let driver_features = (device_features & supported_features).bits();
    header.driver_features_sel.write(0); // driver features [0, 32)
    header.driver_features.write((driver_features & 0xFFFFFFFF) as u32);
    header.driver_features_sel.write(1); // driver features [32, 64)
    header.driver_features.write(((driver_features & 0xFFFFFFFF00000000) >> 32) as u32);

    // read configuration space
    let mut config = unsafe { &mut *((from + 0x100) as *mut VirtIOGpuConfig) };
    info!("Config: {:?}", config);

    // virtio 4.2.4 Legacy interface
    // configure two virtqueues: ingress and egress
    header.guest_page_size.write(PAGE_SIZE as u32); // one page

    let queue_num = 2;
    let mut driver = VirtIOGpu {
        interrupt: node.prop_u32("interrupts").unwrap(),
        interrupt_parent: node.prop_u32("interrupt-parent").unwrap(),
        header: from as usize,
        queue_num,
        queue_address: 0,
        queue_page: [0, 0],
        last_used_idx: 0,
        frame_buffer: 0,
        rect: VirtIOGpuRect::default()
    };

    // 0 for control, 1 for cursor, we use controlq only
    for queue in 0..2 {
        header.queue_sel.write(queue);
        assert_eq!(header.queue_pfn.read(), 0); // not in use
        // 0 for transmit, 1 for receive
        let queue_num_max = header.queue_num_max.read();
        assert!(queue_num_max >= queue_num); // queue available
        let size = virtqueue_size(queue_num as usize, PAGE_SIZE);
        assert!(size % PAGE_SIZE == 0);
        // alloc continuous pages
        let address = unsafe {
            HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(size, PAGE_SIZE).unwrap())
        } as usize;

        debug!("queue {} using page address {:#X} with size {}", queue, address as usize, size);

        header.queue_num.write(queue_num);
        header.queue_align.write(PAGE_SIZE as u32);
        header.queue_pfn.write((address as u32) >> 12);

        if queue == 0 {
            driver.queue_address = address;
            // 0 for transmit, 1 for receive
            for buffer in 0..2 {
                // allocate a page for each buffer
                let page = unsafe {
                    HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(PAGE_SIZE, PAGE_SIZE).unwrap())
                } as usize;
                driver.queue_page[buffer as usize] = page;
                debug!("buffer {} using page address {:#X}", buffer, page as usize);
            }
        }
        header.queue_notify.write(queue);
    }
    header.status.write(VirtIODeviceStatus::DRIVER_OK.bits());

    setup_framebuffer(&mut driver);

    DRIVERS.lock().push(Box::new(driver));
}