//! Memory initialization for aarch64.

use crate::consts::{KERNEL_OFFSET, MEMORY_OFFSET};
use crate::memory::{init_heap, FRAME_ALLOCATOR};
use log::*;
use rcore_memory::PAGE_SIZE;

/// Memory initialization.
pub fn init() {
    init_frame_allocator();
    init_heap();
    info!("memory: init end");
}

fn init_frame_allocator() {
    use bitmap_allocator::BitAlloc;
    use core::ops::Range;

    let end = super::board::probe_memory()
        .expect("failed to find memory map")
        .1;
    let start = _end as usize - KERNEL_OFFSET + MEMORY_OFFSET + PAGE_SIZE;
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
