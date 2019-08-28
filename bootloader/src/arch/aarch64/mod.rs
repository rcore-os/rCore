use aarch64::addr::{PhysAddr, VirtAddr};
use aarch64::paging::{memory_attribute::*, Page, PageTable, PageTableAttribute, PageTableFlags as EF, PhysFrame};
use aarch64::paging::{Size1GiB, Size2MiB, Size4KiB};
use aarch64::{asm::*, barrier, regs::*};
use bcm2837::addr::{phys_to_virt, virt_to_phys, PHYSICAL_IO_BASE};
use core::ptr;
use fixedvec::FixedVec;
use xmas_elf::program::{ProgramHeader64, Type};

const PAGE_SIZE: usize = 4096;
const ALIGN_2MB: u64 = 0x200000;

global_asm!(include_str!("boot.S"));

// TODO: set segments permission
fn create_page_table(start_paddr: usize, end_paddr: usize) {
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

    let block_flags = EF::default_block() | EF::UXN;
    // normal memory
    for frame in PhysFrame::<Size2MiB>::range_of(start_paddr as u64, end_paddr as u64) {
        let paddr = frame.start_address();
        let vaddr = VirtAddr::new(phys_to_virt(paddr.as_u64() as usize) as u64);
        let page = Page::<Size2MiB>::containing_address(vaddr);
        p2[page.p2_index()].set_block::<Size2MiB>(paddr, block_flags, MairNormal::attr_value());
    }
    // device memory
    for frame in PhysFrame::<Size2MiB>::range_of(PHYSICAL_IO_BASE as u64, 0x4000_0000) {
        let paddr = frame.start_address();
        let vaddr = VirtAddr::new(phys_to_virt(paddr.as_u64() as usize) as u64);
        let page = Page::<Size2MiB>::containing_address(vaddr);
        p2[page.p2_index()].set_block::<Size2MiB>(paddr, block_flags | EF::PXN, MairDevice::attr_value());
    }

    p3[0].set_frame(frame_lvl2, EF::default_table(), PageTableAttribute::new(0, 0, 0));
    p3[1].set_block::<Size1GiB>(PhysAddr::new(0x4000_0000), block_flags | EF::PXN, MairDevice::attr_value());

    p4[0].set_frame(frame_lvl3, EF::default_table(), PageTableAttribute::new(0, 0, 0));

    // the bootloader is still running at the lower virtual address range,
    // so the TTBR0_EL1 also needs to be set.
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
        TCR_EL1::A1::UseTTBR0ASID +
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
    // reverse program headers to avoid overlapping in memory copying
    let mut space = alloc_stack!([ProgramHeader64; 32]);
    let mut rev_segments = FixedVec::new(&mut space);
    for i in (0..segments.len()).rev() {
        rev_segments.push(segments[i]).unwrap();
    }

    let (mut start_vaddr, mut end_vaddr) = (VirtAddr::new(core::u64::MAX), VirtAddr::zero());
    for segment in &rev_segments {
        if segment.get_type() != Ok(Type::Load) {
            continue;
        }
        let virt_addr = segment.virtual_addr;
        let offset = segment.offset;
        let file_size = segment.file_size;
        let mem_size = segment.mem_size;

        unsafe {
            let src = (kernel_start as u64 + offset) as *const u8;
            let dst = virt_to_phys(virt_addr as usize) as *mut u8;
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

    create_page_table(0, PHYSICAL_IO_BASE);
    enable_mmu();
}
