use aarch64::addr::{VirtAddr, PhysAddr};
use aarch64::paging::{memory_attribute::*, Page, PageTable, PageTableFlags as EF, PhysFrame};
use aarch64::paging::{Size4KiB, Size2MiB, Size1GiB};
use aarch64::{asm::*, barrier, regs::*};
use core::ptr;
use fixedvec::FixedVec;
use xmas_elf::program::{ProgramHeader64, Type};

const PAGE_SIZE: usize = 4096;
const ALIGN_2MB: u64 = 0x200000;

const IO_REMAP_BASE: u64 = 0x3F00_0000;
const MEMORY_END: u64 = 0x4000_0000;

const RECURSIVE_INDEX: usize = 0o777;
const KERNEL_OFFSET: u64 = 0xFFFF_0000_0000_0000;

global_asm!(include_str!("boot.S"));

fn setup_temp_page_table(start_vaddr: VirtAddr, end_vaddr: VirtAddr, offset: u64) {
    #[repr(align(4096))]
    struct PageData([u8; PAGE_SIZE]);
    static mut PAGE_TABLE_LVL4: PageData = PageData([0; PAGE_SIZE]);
    static mut PAGE_TABLE_LVL3: PageData = PageData([0; PAGE_SIZE]);
    static mut PAGE_TABLE_LVL2: PageData = PageData([0; PAGE_SIZE]);

    let frame_lvl4 = unsafe { PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(&PAGE_TABLE_LVL4 as *const _ as u64)) };
    let frame_lvl3 = unsafe { PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(&PAGE_TABLE_LVL3 as *const _ as u64)) };
    let frame_lvl2 = unsafe { PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(&PAGE_TABLE_LVL2 as *const _ as u64)) };
    let p4 = unsafe { &mut *(frame_lvl4.start_address().as_u64() as *mut PageTable) };
    let p3 = unsafe { &mut *(frame_lvl3.start_address().as_u64() as *mut PageTable) };
    let p2 = unsafe { &mut *(frame_lvl2.start_address().as_u64() as *mut PageTable) };
    p4.zero();
    p3.zero();
    p2.zero();

    let block_flags = EF::VALID | EF::AF | EF::WRITE | EF::UXN;
    // normal memory
    for page in Page::<Size2MiB>::range_of(start_vaddr.as_u64(), end_vaddr.as_u64()) {
        let paddr = PhysAddr::new(page.start_address().as_u64().wrapping_add(offset));
        p2[page.p2_index()].set_block::<Size2MiB>(paddr, block_flags, MairNormal::attr_value());
    }
    // device memory
    for page in Page::<Size2MiB>::range_of(IO_REMAP_BASE, MEMORY_END) {
        let paddr = PhysAddr::new(page.start_address().as_u64());
        p2[page.p2_index()].set_block::<Size2MiB>(paddr, block_flags | EF::PXN, MairDevice::attr_value());
    }

    p3[0].set_frame(frame_lvl2, EF::default(), MairNormal::attr_value());
    p3[1].set_block::<Size1GiB>(PhysAddr::new(MEMORY_END), block_flags | EF::PXN, MairDevice::attr_value());

    p4[0].set_frame(frame_lvl3, EF::default(), MairNormal::attr_value());
    p4[RECURSIVE_INDEX].set_frame(frame_lvl4, EF::default(), MairNormal::attr_value());

    ttbr_el1_write(0, frame_lvl4);
    ttbr_el1_write(1, frame_lvl4);
    tlb_invalidate_all();
}

fn enable_mmu() {
    MAIR_EL1.write(
        MAIR_EL1::Attr0.val(MairNormal::config_value()) +
        MAIR_EL1::Attr1.val(MairDevice::config_value()) +
        MAIR_EL1::Attr2.val(MairNormalNonCacheable::config_value()),
    );

    // Configure various settings of stage 1 of the EL1 translation regime.
    let ips = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);
    TCR_EL1.write(
        TCR_EL1::TBI1::Ignored +
        TCR_EL1::TBI0::Ignored +
        TCR_EL1::AS::Bits_16 +
        TCR_EL1::IPS.val(ips) +

        TCR_EL1::TG1::KiB_4 +
        TCR_EL1::SH1::Inner +
        TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable +
        TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable +
        TCR_EL1::EPD1::EnableTTBR1Walks +
        TCR_EL1::A1::UseTTBR1ASID +
        TCR_EL1::T1SZ.val(16) +

        TCR_EL1::TG0::KiB_4 +
        TCR_EL1::SH0::Inner +
        TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable +
        TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable +
        TCR_EL1::EPD0::EnableTTBR0Walks +
        TCR_EL1::T0SZ.val(16),
    );

    // Switch the MMU on.
    //
    // First, force all previous changes to be seen before the MMU is enabled.
    unsafe { barrier::isb(barrier::SY) }

    // Enable the MMU and turn on data and instruction caching.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);

    // Force MMU init to complete before next instruction
    unsafe { barrier::isb(barrier::SY) }
}

pub fn map_kernel(kernel_start: usize, segments: &FixedVec<ProgramHeader64>) {
    let (mut start_vaddr, mut end_vaddr) = (VirtAddr::new(core::u64::MAX), VirtAddr::zero());
    for segment in segments {
        if segment.get_type() != Ok(Type::Load) {
            continue;
        }
        let virt_addr = segment.virtual_addr;
        let offset = segment.offset;
        let file_size = segment.file_size;
        let mem_size = segment.mem_size;

        unsafe {
            let src = (kernel_start as u64 + offset) as *const u8;
            let dst = virt_addr.wrapping_sub(KERNEL_OFFSET) as *mut u8;
            ptr::copy(src, dst, file_size as usize);
            ptr::write_bytes(dst.offset(file_size as isize), 0, (mem_size - file_size) as usize);
        }

        if virt_addr < start_vaddr.as_u64() {
            start_vaddr = VirtAddr::new(virt_addr).align_down(ALIGN_2MB);
        }
        if virt_addr + mem_size > end_vaddr.as_u64() {
            end_vaddr = VirtAddr::new(virt_addr + mem_size).align_up(ALIGN_2MB);
        }
    }

    setup_temp_page_table(start_vaddr, end_vaddr, KERNEL_OFFSET.wrapping_neg());
    enable_mmu();
}
