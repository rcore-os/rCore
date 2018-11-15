use core::slice;
use memory::{active_table, FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet, Stack};
use super::riscv::{addr::*, register::sstatus};
use ucore_memory::PAGE_SIZE;

/*
* @brief:
*   Init the mermory management module, allow memory access and set up page table and init heap and frame allocator
*/
pub fn init() {
    #[repr(align(4096))]  // align the PageData struct to 4096 bytes
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_ROOT: PageData = PageData([0; PAGE_SIZE]);

    unsafe { sstatus::set_sum(); }  // Allow user memory access
    let frame = Frame::of_addr(PhysAddr::new(&PAGE_TABLE_ROOT as *const _ as u32));  
    super::paging::setup_page_table(frame); // set up page table
    // initialize heap and Frame allocator
    init_frame_allocator();
    // remap the kernel use 4K page
    remap_the_kernel();
    init_heap();
}

/*
* @brief: 
*   Init frame allocator, here use a BitAlloc implemented by segment tree.
*/
fn init_frame_allocator() {
    use bit_allocator::BitAlloc;
    use core::ops::Range;
    use consts::{MEMORY_OFFSET, MEMORY_END};

    let mut ba = FRAME_ALLOCATOR.lock();
    //use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    // keep memory for the kernel heap and set other physical memory available in BitAlloc 
    //ba.insert(to_range(KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE, MEMORY_END));
    // end here is the end entry of the kernel
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
fn remap_the_kernel() {
    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    // set up kernel stack
    let kstack = Stack {
        top: bootstacktop as usize,
        bottom: bootstack as usize + PAGE_SIZE,
    };
    static mut SPACE: [u8; 0x1000] = [0; 0x1000];
    let mut ms = unsafe { MemorySet::new_from_raw_space(&mut SPACE, kstack) };
    ms.push(MemoryArea::new_identity(0x10000000, 0x10000008, MemoryAttr::default(), "serial"));
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));
    unsafe { ms.activate(); }
    use core::mem::forget;
    forget(ms);
    info!("kernel remap end");
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