use super::{BootInfo, MemoryRegionType};
use crate::memory::{init_heap, FRAME_ALLOCATOR};
use bitmap_allocator::BitAlloc;
use rcore_memory::paging::*;

pub fn init(boot_info: &BootInfo) {
    init_frame_allocator(boot_info);
    init_heap();
    info!("memory: init end");
}

/// Init FrameAllocator and insert all 'Usable' regions from BootInfo.
fn init_frame_allocator(boot_info: &BootInfo) {
    let mut ba = FRAME_ALLOCATOR.lock();
    for region in boot_info.memory_map.iter() {
        if region.region_type == MemoryRegionType::Usable {
            ba.insert(
                region.range.start_frame_number as usize..region.range.end_frame_number as usize,
            );
        }
    }
}
