use crate::consts::KERNEL_OFFSET;
use bitmap_allocator::BitAlloc;
// Depends on kernel
use super::{BootInfo, MemoryRegionType};
use crate::memory::{active_table, alloc_frame, init_heap, FRAME_ALLOCATOR};
use crate::HEAP_ALLOCATOR;
use alloc::vec::Vec;
use log::*;
use once::*;
use rcore_memory::paging::*;
use rcore_memory::PAGE_SIZE;

pub fn init(boot_info: &BootInfo) {
    assert_has_not_been_called!("memory::init must be called only once");
    init_frame_allocator(boot_info);
    init_device_vm_map();
    init_heap();
    enlarge_heap();
    info!("memory: init end");
}

/// Init FrameAllocator and insert all 'Usable' regions from BootInfo.
fn init_frame_allocator(boot_info: &BootInfo) {
    let mut ba = FRAME_ALLOCATOR.lock();
    for region in boot_info.memory_map.iter() {
        if region.region_type == MemoryRegionType::Usable {
            ba.insert(
                region.range.start_frame_number as usize..region.range.end_frame_number as usize,
            );
        }
    }
}

fn init_device_vm_map() {
    let mut page_table = active_table();
    // IOAPIC
    page_table
        .map(KERNEL_OFFSET + 0xfec00000, 0xfec00000)
        .update();
    // LocalAPIC
    page_table
        .map(KERNEL_OFFSET + 0xfee00000, 0xfee00000)
        .update();
}

fn enlarge_heap() {
    let mut page_table = active_table();
    let mut addrs = Vec::new();
    let va_offset = KERNEL_OFFSET + 0xe0000000;
    for i in 0..16384 {
        let page = alloc_frame().unwrap();
        let va = KERNEL_OFFSET + 0xe0000000 + page;
        if let Some((ref mut addr, ref mut len)) = addrs.last_mut() {
            if *addr - PAGE_SIZE == va {
                *len += PAGE_SIZE;
                *addr -= PAGE_SIZE;
                continue;
            }
        }
        addrs.push((va, PAGE_SIZE));
    }
    for (addr, len) in addrs.into_iter() {
        for va in (addr..(addr + len)).step_by(PAGE_SIZE) {
            page_table.map(va, va - va_offset).update();
        }
        info!("Adding {:#X} {:#X} to heap", addr, len);
        unsafe {
            HEAP_ALLOCATOR.lock().init(addr, len);
        }
    }
}
