//! Memory initialization for aarch64.

use bit_allocator::BitAlloc;
use ucore_memory::PAGE_SIZE;
use memory::{FRAME_ALLOCATOR, init_heap};
use super::atags::atags::Atags;
//use super::super::HEAP_ALLOCATOR;
use aarch64::{barrier, regs::*, addr::*};
use aarch64::paging::{FrameAllocator, FrameDeallocator, Page, PageRange, PhysFrame as Frame, Size4KiB};
use core::ops::Range;

/// Memory initialization.
pub fn init() {
    /*let (start, end) = memory_map().expect("failed to find memory map");
    unsafe {
        HEAP_ALLOCATOR.lock().init(start, end - start);
    }*/



    #[repr(align(4096))]
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_ROOT: PageData = PageData([0; PAGE_SIZE]);

    let frame = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_ROOT as *const _ as u64));
    super::paging::setup_page_table(frame);

    init_mmu();

    init_frame_allocator();
    init_heap();

    let (start, end) = memory_map().expect("failed to find memory map");
    let mut v = vec![];
    for i in 0..(20 + (start & 0xf)) {
       v.push(i);
       println!("{:x?} {:x?}", &v[i] as * const _ as usize, v);
    }

}

extern "C" {
    static _end: u8;
}


fn init_frame_allocator() {
    use consts::{MEMORY_OFFSET};
    let (start, end) = memory_map().expect("failed to find memory map");
    info!("{:x?} {:x?}", start, end);

    let mut ba = FRAME_ALLOCATOR.lock();
    use core::mem::size_of;
    use ::memory::FrameAlloc;
    info!("{:x?} {:x?}", &FRAME_ALLOCATOR as *const _ as usize, size_of::<FrameAlloc>());
    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    info!("{:x?} {:x?}", KERNEL_HEAP_OFFSET + KERNEL_HEAP_SIZE, end);
    ba.insert(to_range(start, end));
    info!("FrameAllocator init end");

    fn to_range(start: usize, end: usize) -> Range<usize> {
        let page_start = (start - MEMORY_OFFSET) / PAGE_SIZE;
        let page_end = (end - MEMORY_OFFSET - 1) / PAGE_SIZE + 1;
        // info!("{:x?} {:x?}", page_start, page_end);
        page_start..page_end
    }
}

fn init_mmu() {
    info!("init_mmu");

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
    info!("{:x?}", ips);
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

    info!("{:x?}", TCR_EL1.get());
    // Enable the MMU and turn on data and instruction caching.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction
    unsafe { barrier::isb(barrier::SY); }

    info!("mmu enabled!");
}

/// Returns the (start address, end address) of the available memory on this
/// system if it can be determined. If it cannot, `None` is returned.
///
/// This function is expected to return `Some` under all normal cirumstances.
pub fn memory_map() -> Option<(usize, usize)> {
    let binary_end = unsafe { (&_end as *const u8) as u32 };

    let mut atags: Atags = Atags::get();
    while let Some(atag) = atags.next() {
        if let Some(mem) = atag.mem() {
            return Some((binary_end as usize, (mem.start + mem.size) as usize));
        }
    }

    None
}

