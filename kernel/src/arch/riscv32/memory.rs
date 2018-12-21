use core::{slice, mem};
use riscv::{addr::*, register::sstatus};
use ucore_memory::PAGE_SIZE;
use log::*;
use crate::memory::{active_table, FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet, MEMORY_ALLOCATOR};
use crate::consts::{MEMORY_OFFSET, MEMORY_END};

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
    #[repr(align(4096))]  // align the PageData struct to 4096 bytes
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_ROOT: PageData = PageData([0; PAGE_SIZE]);

    unsafe { sstatus::set_sum(); }  // Allow user memory access
    let frame = Frame::of_addr(PhysAddr::new(&PAGE_TABLE_ROOT as *const _ as usize));
    super::paging::setup_page_table(frame); // set up page table
    // initialize heap and Frame allocator
    init_frame_allocator();
    init_heap();
    // remap the kernel use 4K page
    remap_the_kernel();
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

    let mut ba = FRAME_ALLOCATOR.lock();
    ba.insert(to_range(end as usize + PAGE_SIZE, MEMORY_END));
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

/*
* @brief:
*   remmap the kernel memory address with 4K page recorded in p1 page table
*/
#[cfg(not(feature = "no_mmu"))]
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

// First core stores its SATP here.
// Other cores load it later.
static mut SATP: usize = 0;

pub unsafe fn clear_bss() {
    let bss_start = sbss as usize;
    let bss_end = ebss as usize;
    for i in bss_start..bss_end {
        (i as *mut u8).write(0);
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
