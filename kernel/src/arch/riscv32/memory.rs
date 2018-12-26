use core::{slice, mem};
use riscv::{addr::*, register::sstatus};
use ucore_memory::PAGE_SIZE;
use log::*;
use crate::memory::{active_table, FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet, MEMORY_ALLOCATOR};
use crate::consts::{MEMORY_OFFSET, MEMORY_END, KERN_VA_BASE};
use riscv::register::satp;

#[cfg(feature = "no_mmu")]
pub fn init() {
    init_heap();

    let heap_bottom = end as usize;
    let heap_size = MEMORY_END - heap_bottom;
    unsafe { MEMORY_ALLOCATOR.lock().init(heap_bottom, heap_size); }
    info!("available memory: [{:#x}, {:#x})", heap_bottom, MEMORY_END);
}

/*
* @brief:
*   Init the mermory management module, allow memory access and set up page table and init heap and frame allocator
*/
#[cfg(not(feature = "no_mmu"))]
pub fn init() {
    unsafe { sstatus::set_sum(); }  // Allow user memory access
    // initialize heap and Frame allocator
    init_frame_allocator();
    init_heap();
    // remap the kernel use 4K page
    remap_the_kernel();
    loop { }
}

pub fn init_other() {
    unsafe {
        sstatus::set_sum();         // Allow user memory access
        asm!("csrw 0x180, $0; sfence.vma" :: "r"(SATP) :: "volatile");
    }
}

/*
* @brief:
*   Init frame allocator, here use a BitAlloc implemented by segment tree.
*/
fn init_frame_allocator() {
    use bit_allocator::BitAlloc;
    use core::ops::Range;

    // TODO: delete debug code
    let mut ba = FRAME_ALLOCATOR.lock();
    let range = to_range((end as usize) - KERN_VA_BASE + PAGE_SIZE, MEMORY_END);
    info!("FrameAllocator insert {} .. {}", range.start, range.end);
    ba.insert(range);
    info!("FrameAllocator init end");
    // DEBUG: trace code
    trace!("init_frame_allocator: alloc={:x?}", ba.alloc());

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
        assert!(page_start < page_end, "illegal range for frame allocator");
        page_start..page_end
    }
}

/// Remap the kernel memory address with 4K page recorded in p1 page table
#[cfg(all(target_arch = "riscv32", not(feature = "no_mmu")))]
fn remap_the_kernel() {
    let mut ms = MemorySet::new_bare();
    #[cfg(feature = "no_bbl")]
    ms.push(MemoryArea::new_identity(0x10000000, 0x10000008, MemoryAttr::default(), "serial"));
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    ms.push(MemoryArea::new_identity(bootstack as usize, bootstacktop as usize, MemoryAttr::default(), "stack"));
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));
    unsafe { ms.activate(); }
    unsafe { SATP = ms.token(); }
    mem::forget(ms);
    info!("kernel remap end");
}

#[cfg(all(target_arch = "riscv64", not(feature = "no_mmu")))]
fn remap_the_kernel() {
    error!("remap the kernel begin, satp: {:x}", satp::read().bits());
    let mut ms = MemorySet::new_bare();
    info!("ms new bare");
    #[cfg(feature = "no_bbl")]
    ms.push(MemoryArea::new_identity(0x0000_0000_1000_0000, 0x0000_0000_1000_0008, MemoryAttr::default(), "serial"));
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    info!("ms new ident text");
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    info!("ms new ident data");
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    info!("ms new ident rodata");
    ms.push(MemoryArea::new_identity(bootstack as usize, bootstacktop as usize, MemoryAttr::default(), "stack"));
    info!("ms new ident rodatistack");
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));
    info!("ms push finish");
    unsafe { ms.activate(); }
    info!("ms activate finish");
    unsafe { SATP = ms.token(); }
    info!("satp token ok");
    mem::forget(ms);
    error!("kernel remap end");
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
extern {
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
