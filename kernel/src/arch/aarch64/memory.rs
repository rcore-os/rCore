//! Memory initialization for aarch64.

use crate::consts::MEMORY_OFFSET;
use crate::memory::{init_heap, virt_to_phys, FRAME_ALLOCATOR};
use aarch64::{asm::ttbr_el1_write, paging::frame::PhysFrame};
use bootinfo::BootInfo;
use log::*;
use rcore_memory::PAGE_SIZE;

/// Memory initialization.
pub fn init(boot_info: &BootInfo) {
    init_frame_allocator(boot_info);
    init_heap();
    info!("memory: init end");
}

fn init_frame_allocator(boot_info: &BootInfo) {
    use bitmap_allocator::BitAlloc;
    use core::ops::Range;

    let end = boot_info.physical_memory_end;
    let start = virt_to_phys(_end as usize + PAGE_SIZE);
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

#[allow(dead_code)]
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
