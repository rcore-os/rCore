use core::{slice, mem};
use memory::{active_table, FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet};
use super::riscv::{addr::*, register::sstatus};
use ucore_memory::PAGE_SIZE;

pub fn init() {
    #[repr(align(4096))]
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_ROOT: PageData = PageData([0; PAGE_SIZE]);

    unsafe { sstatus::set_sum(); }  // Allow user memory access
    let frame = Frame::of_addr(PhysAddr::new(&PAGE_TABLE_ROOT as *const _ as u32));
    super::paging::setup_page_table(frame);
    init_frame_allocator();
    init_heap();
    remap_the_kernel();
}

pub fn init_other() {
    unsafe {
        sstatus::set_sum();         // Allow user memory access
        asm!("csrw 0x180, $0; sfence.vma" :: "r"(SATP) :: "volatile");
    }
}

fn init_frame_allocator() {
    use bit_allocator::BitAlloc;
    use core::ops::Range;
    use consts::{MEMORY_OFFSET, MEMORY_END};

    let mut ba = FRAME_ALLOCATOR.lock();
    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    ba.insert(to_range(KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE, MEMORY_END));
    info!("FrameAllocator init end");

    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        page_start..page_end
    }
}

fn remap_the_kernel() {
    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    let mut ms = MemorySet::new_bare();
    ms.push(MemoryArea::new_identity(0x10000000, 0x10000008, MemoryAttr::default(), "serial"));
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));
    unsafe { ms.activate(); }
    unsafe { SATP = ms.token(); }
    mem::forget(ms);
    info!("kernel remap end");
}

// First core stores its SATP here.
// Other cores load it later.
static mut SATP: usize = 0;

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