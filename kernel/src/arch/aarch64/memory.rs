//! Memory initialization for aarch64.

use ucore_memory::PAGE_SIZE;
use memory::{FRAME_ALLOCATOR, init_heap, MemoryArea, MemoryAttr, MemorySet, Stack};
use super::atags::atags::Atags;
use aarch64::{barrier, regs::*, addr::*, paging::PhysFrame as Frame};

/// Memory initialization.
pub fn init() {
    #[repr(align(4096))]
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_LVL4: PageData = PageData([0; PAGE_SIZE]);
    static PAGE_TABLE_LVL3: PageData = PageData([0; PAGE_SIZE]);
    static PAGE_TABLE_LVL2: PageData = PageData([0; PAGE_SIZE]);

    let frame_lvl4 = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_LVL4 as *const _ as u64));
    let frame_lvl3 = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_LVL3 as *const _ as u64));
    let frame_lvl2 = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_LVL2 as *const _ as u64));
    super::paging::setup_page_table(frame_lvl4, frame_lvl3, frame_lvl2);

    init_mmu();
    init_frame_allocator();
    init_heap();
    remap_the_kernel();

    info!("memory: init end");
}

fn init_frame_allocator() {
    use bit_allocator::BitAlloc;
    use core::ops::Range;
    use consts::{MEMORY_OFFSET};

    let (start, end) = memory_map().expect("failed to find memory map");
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

fn init_mmu() {
    // device.
    MAIR_EL1.write(
        // Attribute 1
        MAIR_EL1::Attr1_HIGH::Device
            + MAIR_EL1::Attr1_LOW_DEVICE::Device_nGnRE
            // Attribute 0
            + MAIR_EL1::Attr0_HIGH::Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc
            + MAIR_EL1::Attr0_LOW_MEMORY::InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc,
    );
    // Configure various settings of stage 1 of the EL1 translation regime.
    let ips = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);
    TCR_EL1.write(
        TCR_EL1::TBI0::Ignored
            + TCR_EL1::IPS.val(ips)
            + TCR_EL1::TG0::KiB_4 // 4 KiB granule
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::T0SZ.val(16), // Start walks at level 2
    );

    // Switch the MMU on.
    //
    // First, force all previous changes to be seen before the MMU is enabled.
    unsafe { barrier::isb(barrier::SY); }

    // Enable the MMU and turn on data and instruction caching.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction
    unsafe { barrier::isb(barrier::SY); }

    info!("mmu enabled");
}

fn remap_the_kernel() {
    let (bottom, top) = (0, bootstacktop as usize);
    let kstack = Stack {
        top,
        bottom,
    };
    static mut SPACE: [u8; 0x1000] = [0; 0x1000];
    let mut ms = unsafe { MemorySet::new_from_raw_space(&mut SPACE, kstack) };
    ms.push(MemoryArea::new_identity(bottom, top, MemoryAttr::default(), "kstack"));
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));

    // ensure the level 2 page table exists
    ms.push(MemoryArea::new_identity(0x40000000, 0x40200000, MemoryAttr::default(), "arm_control"));
    super::paging::remap_device_2mib(&mut ms, 0x3F000000, 0x40200000);

    unsafe { ms.activate(); }
    use core::mem::forget;
    forget(ms);
    info!("kernel remap end");
}

/// Returns the (start address, end address) of the available memory on this
/// system if it can be determined. If it cannot, `None` is returned.
///
/// This function is expected to return `Some` under all normal cirumstances.
fn memory_map() -> Option<(usize, usize)> {
    let binary_end = unsafe { _end as u32 };

    let mut atags: Atags = Atags::get();
    while let Some(atag) = atags.next() {
        if let Some(mem) = atag.mem() {
            return Some((binary_end as usize, (mem.start + mem.size) as usize));
        }
    }

    None
}

extern {
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
