use alloc::alloc::{GlobalAlloc, Layout};
use alloc::{vec, vec::Vec};
use core::mem::size_of;
use core::slice;
use core::sync::atomic::{fence, Ordering};

use bitflags::*;
use device_tree::util::SliceRead;
use device_tree::Node;
use log::*;
use rcore_memory::PAGE_SIZE;
use volatile::{ReadOnly, Volatile, WriteOnly};

use crate::HEAP_ALLOCATOR;

use super::super::block::virtio_blk;
use super::super::gpu::virtio_gpu;
use super::super::input::virtio_input;
use super::super::net::virtio_net;
use crate::memory::{phys_to_virt, virt_to_phys};

// virtio 4.2.4 Legacy interface
#[repr(C)]
#[derive(Debug)]
pub struct VirtIOHeader {
    magic: ReadOnly<u32>,                    // 0x000
    version: ReadOnly<u32>,                  // 0x004
    device_id: ReadOnly<u32>,                // 0x008
    vendor_id: ReadOnly<u32>,                // 0x00c
    pub device_features: ReadOnly<u32>,      // 0x010
    pub device_features_sel: WriteOnly<u32>, // 0x014
    __r1: [ReadOnly<u32>; 2],
    pub driver_features: WriteOnly<u32>,     // 0x020
    pub driver_features_sel: WriteOnly<u32>, // 0x024
    pub guest_page_size: WriteOnly<u32>,     // 0x028
    __r2: ReadOnly<u32>,
    pub queue_sel: WriteOnly<u32>,    // 0x030
    pub queue_num_max: ReadOnly<u32>, // 0x034
    pub queue_num: WriteOnly<u32>,    // 0x038
    pub queue_align: WriteOnly<u32>,  // 0x03c
    pub queue_pfn: Volatile<u32>,     // 0x040
    queue_ready: Volatile<u32>,       // new interface only
    __r3: [ReadOnly<u32>; 2],
    pub queue_notify: WriteOnly<u32>, // 0x050
    __r4: [ReadOnly<u32>; 3],
    pub interrupt_status: ReadOnly<u32>, // 0x060
    pub interrupt_ack: WriteOnly<u32>,   // 0x064
    __r5: [ReadOnly<u32>; 2],
    pub status: Volatile<u32>, // 0x070
    __r6: [ReadOnly<u32>; 3],
    queue_desc_low: WriteOnly<u32>, // new interface only since here
    queue_desc_high: WriteOnly<u32>,
    __r7: [ReadOnly<u32>; 2],
    queue_avail_low: WriteOnly<u32>,
    queue_avail_high: WriteOnly<u32>,
    __r8: [ReadOnly<u32>; 2],
    queue_used_low: WriteOnly<u32>,
    queue_used_high: WriteOnly<u32>,
    __r9: [ReadOnly<u32>; 21],
    config_generation: ReadOnly<u32>,
}

#[repr(C)]
pub struct VirtIOVirtqueue {
    header: usize,
    queue_address: usize,
    queue_num: usize,
    queue: usize,
    desc: usize,  // *mut VirtIOVirtqueueDesc,
    avail: usize, // *mut VirtIOVirtqueueAvailableRing,
    used: usize,  // *mut VirtIOVirtqueueUsedRing,
    desc_state: Vec<usize>,
    num_used: usize,
    free_head: usize,
    avail_idx: u16,
    last_used_idx: u16,
}

impl VirtIOVirtqueue {
    // Initialize a virtqueue
    pub fn new(header: &mut VirtIOHeader, queue: usize, queue_num: usize) -> VirtIOVirtqueue {
        header.queue_sel.write(queue as u32);
        assert_eq!(header.queue_pfn.read(), 0); // not in use
        let queue_num_max = header.queue_num_max.read();
        assert!(queue_num_max >= queue_num as u32); // queue available
        assert_eq!(queue_num & (queue_num - 1), 0); // power of two
        let align = PAGE_SIZE;
        let size = virtqueue_size(queue_num, align);
        assert_eq!(size % align, 0);
        // alloc continuous pages
        let address =
            unsafe { HEAP_ALLOCATOR.alloc_zeroed(Layout::from_size_align(size, align).unwrap()) }
                as usize;

        header.queue_num.write(queue_num as u32);
        header.queue_align.write(align as u32);
        header.queue_pfn.write((virt_to_phys(address) as u32) >> 12);

        // link desc together
        let desc =
            unsafe { slice::from_raw_parts_mut(address as *mut VirtIOVirtqueueDesc, queue_num) };
        for i in 0..(queue_num - 1) {
            desc[i].next.write((i + 1) as u16);
        }

        VirtIOVirtqueue {
            header: header as *mut VirtIOHeader as usize,
            queue_address: address,
            queue_num,
            queue,
            desc: address,
            avail: address + size_of::<VirtIOVirtqueueDesc>() * queue_num,
            used: address + virtqueue_used_elem_offset(queue_num, align),
            desc_state: vec![0; queue_num],
            num_used: 0,
            free_head: 0,
            avail_idx: 0,
            last_used_idx: 0,
        }
    }

    pub fn can_add(&self, input_len: usize, output_len: usize) -> bool {
        return input_len + output_len + self.num_used <= self.queue_num;
    }

    // Add buffers to the virtqueue
    // Return true on success, false otherwise
    // ref. linux virtio_ring.c virtqueue_add
    pub fn add(&mut self, input: &[&[u8]], output: &[&[u8]], user_data: usize) -> bool {
        assert!(input.len() + output.len() > 0);
        if !self.can_add(input.len(), output.len()) {
            return false;
        }

        let desc = unsafe {
            slice::from_raw_parts_mut(self.desc as *mut VirtIOVirtqueueDesc, self.queue_num)
        };
        let head = self.free_head;
        let mut prev = 0;
        let mut cur = self.free_head;
        for i in 0..output.len() {
            desc[cur].flags.write(VirtIOVirtqueueFlag::NEXT.bits());
            desc[cur]
                .addr
                .write(virt_to_phys(output[i].as_ptr() as usize) as u64);
            desc[cur].len.write(output[i].len() as u32);
            prev = cur;
            cur = desc[cur].next.read() as usize;
        }
        for i in 0..input.len() {
            desc[cur]
                .flags
                .write((VirtIOVirtqueueFlag::NEXT | VirtIOVirtqueueFlag::WRITE).bits());
            desc[cur]
                .addr
                .write(virt_to_phys(input[i].as_ptr() as usize) as u64);
            desc[cur].len.write(input[i].len() as u32);
            prev = cur;
            cur = desc[cur].next.read() as usize;
        }
        desc[prev]
            .flags
            .write(desc[prev].flags.read() & !(VirtIOVirtqueueFlag::NEXT.bits()));

        self.num_used += input.len() + output.len();
        self.free_head = cur;

        let avail = unsafe { &mut *(self.avail as *mut VirtIOVirtqueueAvailableRing) };

        let avail_slot = self.avail_idx as usize & (self.queue_num - 1);
        avail.ring[avail_slot].write(head as u16);

        // write barrier
        fence(Ordering::SeqCst);

        self.avail_idx = self.avail_idx.wrapping_add(1);
        avail.idx.write(self.avail_idx);
        self.desc_state[head] = user_data;
        return true;
    }

    // Add buffers to the virtqueue and notify device about it
    pub fn add_and_notify(&mut self, input: &[&[u8]], output: &[&[u8]], user_data: usize) -> bool {
        let res = self.add(input, output, user_data);
        if res {
            self.notify();
        }
        return res;
    }

    pub fn can_get(&self) -> bool {
        let used = unsafe { &mut *(self.used as *mut VirtIOVirtqueueUsedRing) };
        return self.last_used_idx != used.idx.read();
    }

    // Get device used buffers (input, output, length, user_data)
    // ref. linux virtio_ring.c virtqueue_get_buf_ctx
    pub fn get(&mut self) -> Option<(Vec<&'static [u8]>, Vec<&'static [u8]>, usize, usize)> {
        let used = unsafe { &mut *(self.used as *mut VirtIOVirtqueueUsedRing) };
        if self.last_used_idx == used.idx.read() {
            return None;
        }
        // read barrier
        fence(Ordering::SeqCst);

        let last_used_slot = self.last_used_idx as usize & (self.queue_num - 1);
        let index = used.ring[last_used_slot].id.read() as usize;
        let len = used.ring[last_used_slot].len.read();

        let user_data = self.desc_state[last_used_slot];
        self.desc_state[last_used_slot] = 0;

        let mut cur = index;
        let desc = unsafe {
            slice::from_raw_parts_mut(self.desc as *mut VirtIOVirtqueueDesc, self.queue_num)
        };
        let mut input = Vec::new();
        let mut output = Vec::new();
        loop {
            let flags = VirtIOVirtqueueFlag::from_bits_truncate(desc[cur].flags.read());
            let addr = phys_to_virt(desc[cur].addr.read() as usize);
            let buffer =
                unsafe { slice::from_raw_parts(addr as *const u8, desc[cur].len.read() as usize) };
            if flags.contains(VirtIOVirtqueueFlag::WRITE) {
                input.push(buffer);
            } else {
                output.push(buffer);
            }

            if flags.contains(VirtIOVirtqueueFlag::NEXT) {
                cur = desc[cur].next.read() as usize;
                self.num_used -= 1;
            } else {
                desc[cur].next.write(self.free_head as u16);
                self.num_used -= 1;
                break;
            }
        }

        self.free_head = index;
        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        Some((input, output, len as usize, user_data))
    }

    // Get device used buffers until succeed
    // See get() above
    pub fn get_block(&mut self) -> (Vec<&'static [u8]>, Vec<&'static [u8]>, usize, usize) {
        loop {
            let res = self.get();
            if res.is_some() {
                return res.unwrap();
            }
        }
    }

    // Notify device about new buffers
    pub fn notify(&mut self) {
        let header = unsafe { &mut *(self.header as *mut VirtIOHeader) };
        header.queue_notify.write(self.queue as u32);
    }
}

pub const VIRTIO_CONFIG_SPACE_OFFSET: usize = 0x100;

impl VirtIOHeader {
    pub fn read_device_features(&mut self) -> u64 {
        let mut device_features_bits: u64;
        self.device_features_sel.write(0); // device features [0, 32)
        device_features_bits = self.device_features.read().into();
        self.device_features_sel.write(1); // device features [32, 64)
        device_features_bits = device_features_bits + ((self.device_features.read() as u64) << 32);
        device_features_bits
    }

    pub fn write_driver_features(&mut self, driver_features: u64) {
        self.driver_features_sel.write(0); // driver features [0, 32)
        self.driver_features
            .write((driver_features & 0xFFFFFFFF) as u32);
        self.driver_features_sel.write(1); // driver features [32, 64)
        self.driver_features
            .write(((driver_features & 0xFFFFFFFF00000000) >> 32) as u32);
    }
}

bitflags! {
    pub struct VirtIODeviceStatus : u32 {
        const ACKNOWLEDGE = 1;
        const DRIVER = 2;
        const FAILED = 128;
        const FEATURES_OK = 8;
        const DRIVER_OK = 4;
        const DEVICE_NEEDS_RESET = 64;
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct VirtIOVirtqueueDesc {
    pub addr: Volatile<u64>,
    pub len: Volatile<u32>,
    pub flags: Volatile<u16>,
    pub next: Volatile<u16>,
}

bitflags! {
    pub struct VirtIOVirtqueueFlag : u16 {
        const NEXT = 1;
        const WRITE = 2;
        const INDIRECT = 4;
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct VirtIOVirtqueueAvailableRing {
    pub flags: Volatile<u16>,
    pub idx: Volatile<u16>,
    pub ring: [Volatile<u16>; 32], // actual size: queue_size
    used_event: Volatile<u16>,     // unused
}

#[repr(C)]
#[derive(Debug)]
pub struct VirtIOVirtqueueUsedElem {
    id: Volatile<u32>,
    len: Volatile<u32>,
}

#[repr(C)]
#[derive(Debug)]
pub struct VirtIOVirtqueueUsedRing {
    pub flags: Volatile<u16>,
    pub idx: Volatile<u16>,
    pub ring: [VirtIOVirtqueueUsedElem; 32], // actual size: queue_size
    avail_event: Volatile<u16>,              // unused
}

// virtio 2.4.2 Legacy Interfaces: A Note on Virtqueue Layout
pub fn virtqueue_size(num: usize, align: usize) -> usize {
    (((size_of::<VirtIOVirtqueueDesc>() * num + size_of::<u16>() * (3 + num)) + align)
        & !(align - 1))
        + (((size_of::<u16>() * 3 + size_of::<VirtIOVirtqueueUsedElem>() * num) + align)
            & !(align - 1))
}

pub fn virtqueue_used_elem_offset(num: usize, align: usize) -> usize {
    ((size_of::<VirtIOVirtqueueDesc>() * num + size_of::<u16>() * (3 + num)) + align) & !(align - 1)
}

pub fn virtio_probe(node: &Node) {
    if let Some(reg) = node.prop_raw("reg") {
        let paddr = reg.as_slice().read_be_u64(0).unwrap();
        let vaddr = phys_to_virt(paddr as usize);
        debug!("walk dt {:x} {:x}", paddr, vaddr);
        let size = reg.as_slice().read_be_u64(8).unwrap();
        // assuming one page
        assert_eq!(size as usize, PAGE_SIZE);
        let header = unsafe { &mut *(vaddr as *mut VirtIOHeader) };
        let magic = header.magic.read();
        let version = header.version.read();
        let device_id = header.device_id.read();
        // only support legacy device
        if magic == 0x74726976 && version == 1 && device_id != 0 {
            // "virt" magic
            info!(
                "Detected virtio device with vendor id {:#X}",
                header.vendor_id.read()
            );
            info!("Device tree node {:?}", node);
            // virtio 3.1.1 Device Initialization
            header.status.write(0);
            header.status.write(VirtIODeviceStatus::ACKNOWLEDGE.bits());
            match device_id {
                1 => virtio_net::virtio_net_init(node),
                2 => virtio_blk::virtio_blk_init(node),
                16 => virtio_gpu::virtio_gpu_init(node),
                18 => virtio_input::virtio_input_init(node),
                _ => warn!("Unrecognized virtio device {}", device_id),
            }
        }
    }
}
