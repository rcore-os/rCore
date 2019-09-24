use aarch64::paging::{
    memory_attribute::*, PageTable, PageTableAttribute as Attr, PageTableFlags as EF,
};
use aarch64::paging::{Page, PhysFrame, Size1GiB, Size2MiB, Size4KiB};
use aarch64::{addr::PhysAddr, asm, barrier, regs::*};
use bcm2837::addr::{phys_to_virt, PHYSICAL_IO_END, PHYSICAL_IO_START};

const PAGE_SIZE: usize = 4096;

global_asm!(include_str!("entry.S"));

#[link_section = ".text.boot"]
fn map_2mib(p2: &mut PageTable, start: usize, end: usize, flag: EF, attr: Attr) {
    for frame in PhysFrame::<Size2MiB>::range_of(start as u64, end as u64) {
        let paddr = frame.start_address();
        let page = Page::<Size2MiB>::of_addr(phys_to_virt(paddr.as_u64() as usize) as u64);
        p2[page.p2_index()].set_block::<Size2MiB>(paddr, flag, attr);
    }
}

#[no_mangle]
#[link_section = ".text.boot"]
extern "C" fn create_init_paging() {
    let frame_lvl4 = PhysFrame::<Size4KiB>::of_addr(page_table_lvl4 as u64);
    let frame_lvl3 = PhysFrame::<Size4KiB>::of_addr(page_table_lvl3 as u64);
    let frame_lvl2 = PhysFrame::<Size4KiB>::of_addr(page_table_lvl2 as u64);
    let p4 = unsafe { &mut *(page_table_lvl4 as *mut PageTable) };
    let p3 = unsafe { &mut *(page_table_lvl3 as *mut PageTable) };
    let p2 = unsafe { &mut *(page_table_lvl2 as *mut PageTable) };
    p4.zero();
    p3.zero();
    p2.zero();

    let block_flags = EF::default_block() | EF::UXN;
    // 0x0000_0000 ~ 0x80_0000_0000
    p4[0].set_frame(frame_lvl3, EF::default_table(), Attr::new(0, 0, 0));

    // 0x0000_0000 ~ 0x4000_000
    p3[0].set_frame(frame_lvl2, EF::default_table(), Attr::new(0, 0, 0));
    // 0x4000_0000 ~ 0x8000_000
    p3[1].set_block::<Size1GiB>(
        PhysAddr::new(PHYSICAL_IO_END as u64),
        block_flags | EF::PXN,
        MairDevice::attr_value(),
    );

    // normal memory (0x0000_0000 ~ 0x3F00_000)
    map_2mib(
        p2,
        0,
        PHYSICAL_IO_START,
        block_flags,
        MairNormal::attr_value(),
    );
    // device memory (0x3F00_000 ~ 0x4000_000)
    map_2mib(
        p2,
        PHYSICAL_IO_START,
        PHYSICAL_IO_END,
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
    asm::ttbr_el1_write(0, frame_lvl4);
    asm::ttbr_el1_write(1, frame_lvl4);

    // Switch the MMU on.
    //
    // First, force all previous changes to be seen before the MMU is enabled.
    unsafe { barrier::isb(barrier::SY) }

    // Enable the MMU and turn on data and instruction caching.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction
    unsafe { barrier::isb(barrier::SY) }
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

#[allow(dead_code)]
extern "C" {
    fn sbss();
    fn ebss();
    fn page_table_lvl4();
    fn page_table_lvl3();
    fn page_table_lvl2();
}
