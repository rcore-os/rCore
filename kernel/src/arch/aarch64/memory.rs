//! Memory initialization for aarch64.

use super::paging::MMIOType;
use crate::consts::{KERNEL_OFFSET, MEMORY_OFFSET};
use crate::memory::{init_heap, Linear, MemoryAttr, MemorySet, FRAME_ALLOCATOR};
use aarch64::regs::*;
use log::*;
use rcore_memory::PAGE_SIZE;

/// Memory initialization.
pub fn init() {
    init_frame_allocator();
    init_heap();
    remap_the_kernel();
    info!("memory: init end");
}

fn init_frame_allocator() {
    use bit_allocator::BitAlloc;
    use core::ops::Range;

    let end = super::board::probe_memory()
        .expect("failed to find memory map")
        .1;
    let start = (_end as u64 + PAGE_SIZE as u64).wrapping_sub(KERNEL_OFFSET as u64) as usize;
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

static mut KERNEL_MEMORY_SET: Option<MemorySet> = None;

/// remap kernel page table after all initialization.
fn remap_the_kernel() {
    let offset = -(KERNEL_OFFSET as isize);
    let mut ms = MemorySet::new_bare();
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

    use super::board::{IO_REMAP_BASE, IO_REMAP_END};
    ms.push(
        IO_REMAP_BASE,
        IO_REMAP_END,
        MemoryAttr::default().mmio(MMIOType::Device as u8),
        Linear::new(offset),
        "io_remap",
    );

    info!("{:#x?}", ms);
    unsafe { ms.get_page_table_mut().activate_as_kernel() }
    unsafe { KERNEL_MEMORY_SET = Some(ms) }
    info!("kernel remap end");
}

pub fn ioremap(paddr: usize, len: usize, name: &'static str) -> usize {
    let offset = -(KERNEL_OFFSET as isize);
    let vaddr = paddr.wrapping_add(KERNEL_OFFSET);
    if let Some(ms) = unsafe { KERNEL_MEMORY_SET.as_mut() } {
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
