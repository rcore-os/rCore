//! Memory initialization for aarch64.

use super::paging::MMIOType;
use crate::consts::{KERNEL_OFFSET, MEMORY_OFFSET};
use crate::memory::{init_heap, kernel_offset, Linear, MemoryAttr, MemorySet, FRAME_ALLOCATOR};
use log::*;
use rcore_memory::PAGE_SIZE;
use spin::Mutex;

static KERNEL_MEMORY_SET: Mutex<Option<MemorySet>> = Mutex::new(None);

/// Memory initialization.
pub fn init() {
    init_frame_allocator();
    init_heap();
    map_kernel();
    info!("memory: init end");
}

pub fn init_other() {
    if let Some(ms) = KERNEL_MEMORY_SET.lock().as_mut() {
        unsafe { ms.get_page_table_mut().activate_as_kernel() };
    }
}

fn init_frame_allocator() {
    use bitmap_allocator::BitAlloc;
    use core::ops::Range;

    let end = super::board::probe_memory()
        .expect("failed to find memory map")
        .1;
    let start = kernel_offset(_end as usize) + MEMORY_OFFSET + PAGE_SIZE;
    let mut ba = FRAME_ALLOCATOR.lock();
    ba.insert(to_range(start, end));
    info!("FrameAllocator init end");

    /// Transform memory area `[start, end)` to integer range for `FrameAllocator`
    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        page_start..page_end
    }
}

/// Create fine-grained mappings for the kernel
fn map_kernel() {
    let offset = -(KERNEL_OFFSET as isize);
    let mut ms = MemorySet::new();
    ms.push(
        stext as usize,
        etext as usize,
        MemoryAttr::default().execute().readonly(),
        Linear::new(offset),
        "text",
    );
    ms.push(
        sdata as usize,
        edata as usize,
        MemoryAttr::default(),
        Linear::new(offset),
        "data",
    );
    ms.push(
        srodata as usize,
        erodata as usize,
        MemoryAttr::default().readonly(),
        Linear::new(offset),
        "rodata",
    );
    ms.push(
        sbss as usize,
        ebss as usize,
        MemoryAttr::default(),
        Linear::new(offset),
        "bss",
    );
    ms.push(
        bootstack as usize,
        bootstacktop as usize,
        MemoryAttr::default(),
        Linear::new(offset),
        "kstack",
    );
    ms.push(
        super::board::PERIPHERALS_START,
        super::board::PERIPHERALS_END,
        MemoryAttr::default().mmio(MMIOType::Device as u8),
        Linear::new(offset),
        "peripherals",
    );

    let page_table = ms.get_page_table_mut();
    page_table.map_physical_memory(0, super::board::PERIPHERALS_START);
    unsafe { page_table.activate_as_kernel() };
    *KERNEL_MEMORY_SET.lock() = Some(ms);

    info!("map kernel end");
}

/// map the I/O memory range into the kernel page table
pub fn ioremap(paddr: usize, len: usize, name: &'static str) -> usize {
    let offset = -(KERNEL_OFFSET as isize);
    let vaddr = paddr.wrapping_add(KERNEL_OFFSET);
    if let Some(ms) = KERNEL_MEMORY_SET.lock().as_mut() {
        ms.push(
            vaddr,
            vaddr + len,
            MemoryAttr::default().mmio(MMIOType::NormalNonCacheable as u8),
            Linear::new(offset),
            name,
        );
        return vaddr;
    }
    0
}

extern "C" {
    fn stext();
    fn etext();
    fn sdata();
    fn edata();
    fn srodata();
    fn erodata();
    fn sbss();
    fn ebss();
    fn bootstack();
    fn bootstacktop();
    fn _start();
    fn _end();
}
