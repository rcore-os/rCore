use super::board::{PERIPHERALS_END, PERIPHERALS_START};
use crate::memory::phys_to_virt;
use aarch64::paging::{memory_attribute::*, PageTableAttribute as Attr, PageTableFlags as EF};
use aarch64::paging::{Page, PageTable, PhysFrame, Size1GiB, Size2MiB, Size4KiB};
use aarch64::{align_down, align_up, PhysAddr, ALIGN_1GIB, ALIGN_2MIB};
use aarch64::{barrier, cache, regs::*, translation};

global_asm!(include_str!("entry.S"));

#[link_section = ".text.boot"]
fn map_2mib(p2: &mut PageTable, start: usize, end: usize, flag: EF, attr: Attr) {
    let aligned_start = align_down(start as u64, ALIGN_2MIB);
    let aligned_end = align_up(end as u64, ALIGN_2MIB);
    for frame in PhysFrame::<Size2MiB>::range_of(aligned_start, aligned_end) {
        let paddr = frame.start_address();
        let page = Page::<Size2MiB>::of_addr(phys_to_virt(paddr.as_u64() as usize) as u64);
        p2[page.p2_index()].set_block::<Size2MiB>(paddr, flag, attr);
    }
}

#[no_mangle]
#[link_section = ".text.boot"]
extern "C" fn create_init_paging() {
    let p4 = unsafe { &mut *(page_table_lvl4 as *mut PageTable) };
    let p3 = unsafe { &mut *(page_table_lvl3 as *mut PageTable) };
    let p2 = unsafe { &mut *(page_table_lvl2 as *mut PageTable) };
    let frame_lvl3 = PhysFrame::<Size4KiB>::of_addr(page_table_lvl3 as u64);
    let frame_lvl2 = PhysFrame::<Size4KiB>::of_addr(page_table_lvl2 as u64);
    p4.zero();
    p3.zero();
    p2.zero();

    let block_flags = EF::default_block() | EF::UXN;
    // 0x0000_0000_0000 ~ 0x0080_0000_0000
    p4[0].set_frame(frame_lvl3, EF::default_table(), Attr::new(0, 0, 0));
    // 0x8000_0000_0000 ~ 0x8080_0000_0000
    p4[256].set_frame(frame_lvl3, EF::default_table(), Attr::new(0, 0, 0));

    // 0x0000_0000 ~ 0x4000_0000
    p3[0].set_frame(frame_lvl2, EF::default_table(), Attr::new(0, 0, 0));
    // 0x4000_0000 ~ 0x8000_0000
    p3[1].set_block::<Size1GiB>(
        PhysAddr::new(PERIPHERALS_END as u64),
        block_flags | EF::PXN,
        MairDevice::attr_value(),
    );

    // normal memory (0x0000_0000 ~ 0x3F00_0000)
    map_2mib(
        p2,
        0,
        PERIPHERALS_START,
        block_flags,
        MairNormal::attr_value(),
    );
    // device memory (0x3F00_0000 ~ 0x4000_0000)
    map_2mib(
        p2,
        PERIPHERALS_START,
        align_down(PERIPHERALS_END as u64, ALIGN_1GIB) as usize,
        block_flags | EF::PXN,
        MairDevice::attr_value(),
    );
}

#[no_mangle]
#[link_section = ".text.boot"]
extern "C" fn enable_mmu() {
    MAIR_EL1.write(
        MAIR_EL1::Attr0.val(MairNormal::config_value())
            + MAIR_EL1::Attr1.val(MairDevice::config_value())
            + MAIR_EL1::Attr2.val(MairNormalNonCacheable::config_value()),
    );

    // Configure various settings of stage 1 of the EL1 translation regime.
    let ips = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);
    TCR_EL1.write(
        TCR_EL1::TBI1::Ignored
            + TCR_EL1::TBI0::Ignored
            + TCR_EL1::AS::Bits_16
            + TCR_EL1::IPS.val(ips)
            + TCR_EL1::TG1::KiB_4
            + TCR_EL1::SH1::Inner
            + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD1::EnableTTBR1Walks
            + TCR_EL1::A1::UseTTBR0ASID
            + TCR_EL1::T1SZ.val(16)
            + TCR_EL1::TG0::KiB_4
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::T0SZ.val(16),
    );

    // Set both TTBR0_EL1 and TTBR1_EL1
    let frame_lvl4 = PhysFrame::<Size4KiB>::of_addr(page_table_lvl4 as u64);
    translation::ttbr_el1_write(0, frame_lvl4);
    translation::ttbr_el1_write(1, frame_lvl4);
    translation::local_invalidate_tlb_all();

    // Enable the MMU and turn on data and instruction caching.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction
    unsafe { barrier::isb() }

    // Invalidate the local I-cache so that any instructions fetched
    // speculatively from the PoC are discarded
    cache::ICache::local_flush_all();
}

#[no_mangle]
#[link_section = ".text.boot"]
extern "C" fn clear_bss() {
    let start = sbss as usize;
    let end = ebss as usize;
    let step = core::mem::size_of::<usize>();
    for i in (start..end).step_by(step) {
        unsafe { (i as *mut usize).write(0) };
    }
}

extern "C" {
    fn sbss();
    fn ebss();
    fn page_table_lvl4();
    fn page_table_lvl3();
    fn page_table_lvl2();
    fn _start();
    fn _end();
}
