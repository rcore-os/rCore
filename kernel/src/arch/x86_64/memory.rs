use super::paging::PageTableImpl;
use crate::memory::FRAME_ALLOCATOR;
use bitmap_allocator::BitAlloc;
use rboot::{BootInfo, MemoryType};
use rcore_memory::paging::*;
use rcore_memory::PAGE_SIZE;

pub fn init(boot_info: &BootInfo) {
    init_frame_allocator(boot_info);
    info!("memory: init end");
}

/// Init FrameAllocator and insert all 'Usable' regions from BootInfo.
fn init_frame_allocator(boot_info: &BootInfo) {
    let mut ba = FRAME_ALLOCATOR.lock();
    for region in boot_info.memory_map.clone().iter {
        if region.ty == MemoryType::CONVENTIONAL {
            let start_frame = region.phys_start as usize / PAGE_SIZE;
            let end_frame = start_frame + region.page_count as usize;
            ba.insert(start_frame..end_frame);
        }
    }
}

/// The method for initializing kernel virtual memory space, a memory space of 512 GiB.
/// The memory space is resided at the 509th item of the first-level page table.
/// After the initialization, mapping on this space will be "broadcast" to all page tables.
pub fn init_kernel_kseg2_map() {
    let mut page_table = unsafe { PageTableImpl::kernel_table() };
    // Dirty hack here:
    // We do not really need the mapping. Indeed, we only need the second-level page table.
    // Second-level page table item can then be copied to all page tables safely.
    // This hack requires the page table not to recycle the second level page table on unmap.

    page_table.map(0xfffffe8000000000, 0x0).update();
    page_table.unmap(0xfffffe8000000000);
}
