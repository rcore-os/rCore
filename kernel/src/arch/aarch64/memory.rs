//! Memory initialization for aarch64.

use ucore_memory::PAGE_SIZE;
use atags::atags::Atags;
use crate::HEAP_ALLOCATOR;
use log::*;

/// Memory initialization.
pub fn init() {
    let (start, end) = memory_map().expect("failed to find memory map");
    unsafe {
        HEAP_ALLOCATOR.lock().init(start, end - start);
    }
    info!("memory: init end");
}

extern "C" {
    static _end: u8;
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
