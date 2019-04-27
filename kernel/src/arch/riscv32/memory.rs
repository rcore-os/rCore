use crate::consts::{KERNEL_OFFSET, MEMORY_END, MEMORY_OFFSET};
use crate::memory::{init_heap, Linear, MemoryAttr, MemorySet, FRAME_ALLOCATOR};
use core::mem;
use log::*;
use rcore_memory::PAGE_SIZE;
use riscv::register::satp;
use riscv::{addr::*, register::sstatus};

/// Initialize the memory management module
pub fn init(dtb: usize) {
    // allow user memory access
    // NOTE: In K210 priv v1.9.1, sstatus.SUM is PUM which has opposite meaning!
    #[cfg(not(feature = "board_k210"))]
    unsafe {
        sstatus::set_sum();
    }
    // initialize heap and Frame allocator
    init_frame_allocator();
    init_heap();
    // remap the kernel use 4K page
    unsafe {
        super::paging::setup_recursive_mapping();
    }
    remap_the_kernel(dtb);
}

pub fn init_other() {
    unsafe {
        sstatus::set_sum(); // Allow user memory access
        asm!("csrw satp, $0; sfence.vma" :: "r"(SATP) :: "volatile");
    }
}

fn init_frame_allocator() {
    use bitmap_allocator::BitAlloc;
    use core::ops::Range;

    let mut ba = FRAME_ALLOCATOR.lock();
    let range = to_range(
        (end as usize) - KERNEL_OFFSET + MEMORY_OFFSET + PAGE_SIZE,
        MEMORY_END,
    );
    ba.insert(range);

    info!("frame allocator: init end");

    /// Transform memory area `[start, end)` to integer range for `FrameAllocator`
    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        assert!(page_start < page_end, "illegal range for frame allocator");
        page_start..page_end
    }
}

/// Remap the kernel memory address with 4K page recorded in p1 page table
fn remap_the_kernel(dtb: usize) {
    let offset = -(KERNEL_OFFSET as isize - MEMORY_OFFSET as isize);
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
        bootstack as usize,
        bootstacktop as usize,
        MemoryAttr::default(),
        Linear::new(offset),
        "stack",
    );
    ms.push(
        sbss as usize,
        ebss as usize,
        MemoryAttr::default(),
        Linear::new(offset),
        "bss",
    );
    // TODO: dtb on rocket chip
    #[cfg(not(feature = "board_rocket_chip"))]
    ms.push(
        dtb,
        dtb + super::consts::MAX_DTB_SIZE,
        MemoryAttr::default().readonly(),
        Linear::new(offset),
        "dts",
    );
    // map PLIC for HiFiveU & VirtIO
    let offset = -(KERNEL_OFFSET as isize);
    ms.push(
        KERNEL_OFFSET + 0x0C00_2000,
        KERNEL_OFFSET + 0x0C00_2000 + PAGE_SIZE,
        MemoryAttr::default(),
        Linear::new(offset),
        "plic0",
    );
    ms.push(
        KERNEL_OFFSET + 0x0C20_2000,
        KERNEL_OFFSET + 0x0C20_2000 + PAGE_SIZE,
        MemoryAttr::default(),
        Linear::new(offset),
        "plic1",
    );
    // map UART for HiFiveU
    ms.push(
        KERNEL_OFFSET + 0x10010000,
        KERNEL_OFFSET + 0x10010000 + PAGE_SIZE,
        MemoryAttr::default(),
        Linear::new(offset),
        "uart",
    );
    // map UART for VirtIO
    ms.push(
        KERNEL_OFFSET + 0x10000000,
        KERNEL_OFFSET + 0x10000000 + PAGE_SIZE,
        MemoryAttr::default(),
        Linear::new(offset),
        "uart16550",
    );
    unsafe {
        ms.activate();
    }
    unsafe {
        SATP = ms.token();
    }
    mem::forget(ms);
    info!("remap kernel end");
}

// First core stores its SATP here.
// Other cores load it later.
static mut SATP: usize = 0;

pub unsafe fn clear_bss() {
    let start = sbss as usize;
    let end = ebss as usize;
    let step = core::mem::size_of::<usize>();
    for i in (start..end).step_by(step) {
        (i as *mut usize).write(0);
    }
}

// Symbols provided by linker script
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
    fn start();
    fn end();
    fn bootstack();
    fn bootstacktop();
}
