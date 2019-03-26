use core::mem;
use riscv::{addr::*, register::sstatus};
use rcore_memory::PAGE_SIZE;
use log::*;
use crate::memory::{FRAME_ALLOCATOR, init_heap, MemoryAttr, MemorySet, Linear};
use crate::consts::{MEMORY_OFFSET, MEMORY_END, KERNEL_OFFSET};
use riscv::register::satp;

/*
* @brief:
*   Init the mermory management module, allow memory access and set up page table and init heap and frame allocator
*/
pub fn init(dtb: usize) {
    unsafe { sstatus::set_sum(); }  // Allow user memory access
    // initialize heap and Frame allocator
    init_frame_allocator();
    init_heap();
    // remap the kernel use 4K page
    remap_the_kernel(dtb);
}

pub fn init_other() {
    unsafe {
        sstatus::set_sum();         // Allow user memory access
        asm!("csrw satp, $0; sfence.vma" :: "r"(SATP) :: "volatile");
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
    let range = to_range((end as usize) - KERNEL_OFFSET + MEMORY_OFFSET + PAGE_SIZE, MEMORY_END);
    ba.insert(range);

    info!("frame allocator: init end");

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
fn remap_the_kernel(dtb: usize) {
    let offset = -(KERNEL_OFFSET as isize - MEMORY_OFFSET as isize);
    let mut ms = MemorySet::new_bare();
    ms.push(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), Linear::new(offset), "text");
    ms.push(sdata as usize, edata as usize, MemoryAttr::default(), Linear::new(offset), "data");
    ms.push(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), Linear::new(offset), "rodata");
    ms.push(bootstack as usize, bootstacktop as usize, MemoryAttr::default(), Linear::new(offset), "stack");
    ms.push(sbss as usize, ebss as usize, MemoryAttr::default(), Linear::new(offset), "bss");
    ms.push(dtb, dtb + super::consts::MAX_DTB_SIZE, MemoryAttr::default().readonly(), Linear::new(offset), "dts");
    // map PLIC for HiFiveU
    let offset = -(KERNEL_OFFSET as isize);
    ms.push(KERNEL_OFFSET + 0x0C00_2000, KERNEL_OFFSET + 0x0C00_2000 + PAGE_SIZE, MemoryAttr::default(), Linear::new(offset), "plic0");
    ms.push(KERNEL_OFFSET + 0x0C20_2000, KERNEL_OFFSET + 0x0C20_2000 + PAGE_SIZE, MemoryAttr::default(), Linear::new(offset), "plic1");
    unsafe { ms.activate(); }
    unsafe { SATP = ms.token(); }
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
