//! Memory initialization for aarch64.

use crate::memory::{init_heap, MemoryArea, MemoryAttr, MemorySet, FRAME_ALLOCATOR};
use super::paging::MMIOType;
use aarch64::paging::{memory_attribute::*, PhysFrame as Frame};
use aarch64::{addr::*, barrier, regs::*};
use atags::atags::Atags;
use lazy_static::lazy_static;
use log::*;
use spin::Mutex;
use ucore_memory::PAGE_SIZE;

/// Memory initialization.
pub fn init() {
    init_frame_allocator();
    init_heap();
    remap_the_kernel();
    info!("memory: init end");
}

/// initialize temporary paging and enable mmu immediately after boot. Serial port is disabled at this time.
pub fn init_mmu_early() {
    #[repr(align(4096))]
    struct PageData([u8; PAGE_SIZE]);
    static PAGE_TABLE_LVL4: PageData = PageData([0; PAGE_SIZE]);
    static PAGE_TABLE_LVL3: PageData = PageData([0; PAGE_SIZE]);
    static PAGE_TABLE_LVL2: PageData = PageData([0; PAGE_SIZE]);

    let frame_lvl4 = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_LVL4 as *const _ as u64));
    let frame_lvl3 = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_LVL3 as *const _ as u64));
    let frame_lvl2 = Frame::containing_address(PhysAddr::new(&PAGE_TABLE_LVL2 as *const _ as u64));
    super::paging::setup_temp_page_table(frame_lvl4, frame_lvl3, frame_lvl2);

    // device.
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

fn init_frame_allocator() {
    use crate::consts::MEMORY_OFFSET;
    use bit_allocator::BitAlloc;
    use core::ops::Range;

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

lazy_static! {
    pub static ref KERNEL_MEMORY_SET: Mutex<MemorySet> = Mutex::new(MemorySet::new_bare());
}

/// remap kernel page table after all initialization.
fn remap_the_kernel() {
    let mut ms = KERNEL_MEMORY_SET.lock();
    ms.push(MemoryArea::new_identity(0, bootstacktop as usize, MemoryAttr::default(), "kstack"));
    ms.push(MemoryArea::new_identity(stext as usize, etext as usize, MemoryAttr::default().execute().readonly(), "text"));
    ms.push(MemoryArea::new_identity(sdata as usize, edata as usize, MemoryAttr::default(), "data"));
    ms.push(MemoryArea::new_identity(srodata as usize, erodata as usize, MemoryAttr::default().readonly(), "rodata"));
    ms.push(MemoryArea::new_identity(sbss as usize, ebss as usize, MemoryAttr::default(), "bss"));

    use super::board::{IO_REMAP_BASE, IO_REMAP_END};
    ms.push(MemoryArea::new_identity(
        IO_REMAP_BASE,
        IO_REMAP_END,
        MemoryAttr::default().mmio(MMIOType::Device as u8),
        "io_remap",
    ));

    unsafe { ms.get_page_table_mut().activate_as_kernel() }
    info!("kernel remap end");
}

pub fn ioremap(start: usize, len: usize, name: &'static str) -> usize {
    let mut ms = KERNEL_MEMORY_SET.lock();
    let area = MemoryArea::new_identity(
        start,
        start + len,
        MemoryAttr::default().mmio(MMIOType::NormalNonCacheable as u8),
        name,
    );
    ms.push(area);
    start
}

/// Returns the (start address, end address) of the available memory on this
/// system if it can be determined. If it cannot, `None` is returned.
///
/// This function is expected to return `Some` under all normal cirumstances.
fn memory_map() -> Option<(usize, usize)> {
    let binary_end = _end as u32;

    let mut atags: Atags = Atags::get();
    while let Some(atag) = atags.next() {
        if let Some(mem) = atag.mem() {
            return Some((binary_end as usize, (mem.start + mem.size) as usize));
        }
    }

    None
}

extern "C" {
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
