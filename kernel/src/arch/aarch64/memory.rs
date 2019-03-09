//! Memory initialization for aarch64.

use crate::memory::{init_heap, Linear, MemoryAttr, MemorySet, FRAME_ALLOCATOR};
use crate::consts::{MEMORY_OFFSET, KERNEL_OFFSET};
use super::paging::MMIOType;
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

    let end = super::board::probe_memory().expect("failed to find memory map").1;
    let start = (_end as u64 + PAGE_SIZE as u64).wrapping_sub(KERNEL_OFFSET as u64) as usize;
    let mut ba = FRAME_ALLOCATOR.lock();
    ba.insert(to_range(start, end));
    info!("FrameAllocator init end");

    /*
     * @param:
     *   start: start address
     *   end: end address
     * @brief:
     *   transform the memory address to the page number
     * @retval:
     *   the page number range from start address to end address
     */
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
    ms.push(KERNEL_OFFSET, bootstacktop as usize, Linear::new(offset, MemoryAttr::default()), "kstack");
    ms.push(stext as usize, etext as usize, Linear::new(offset, MemoryAttr::default().execute().readonly()), "text");
    ms.push(sdata as usize, edata as usize, Linear::new(offset, MemoryAttr::default()), "data");
    ms.push(srodata as usize, erodata as usize, Linear::new(offset, MemoryAttr::default().readonly()), "rodata");
    ms.push(sbss as usize, ebss as usize, Linear::new(offset, MemoryAttr::default()), "bss");

    use super::board::{IO_REMAP_BASE, IO_REMAP_END};
    ms.push(IO_REMAP_BASE, IO_REMAP_END, Linear::new(offset, MemoryAttr::default().mmio(MMIOType::Device as u8)), "io_remap");

    info!("{:#x?}", ms);
    unsafe { ms.get_page_table_mut().activate_as_kernel() }
    unsafe { KERNEL_MEMORY_SET = Some(ms) }
    info!("kernel remap end");
}

pub fn ioremap(paddr: usize, len: usize, name: &'static str) -> usize {
    let offset = -(KERNEL_OFFSET as isize);
    let vaddr = paddr.wrapping_add(KERNEL_OFFSET);
    if let Some(ms) = unsafe { KERNEL_MEMORY_SET.as_mut() } {
        ms.push(vaddr, vaddr + len, Linear::new(offset, MemoryAttr::default().mmio(MMIOType::NormalNonCacheable as u8)), name);
        return vaddr;
    }
    0
}

extern "C" {
    fn bootstacktop();
    fn stext();
    fn etext();
    fn sdata();
    fn edata();
    fn srodata();
    fn erodata();
    fn sbss();
    fn ebss();
    fn _start();
    fn _end();
}
