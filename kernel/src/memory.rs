pub use arch::paging::*;
use bit_allocator::{BitAlloc, BitAlloc64K};
use consts::MEMORY_OFFSET;
use spin::{Mutex, MutexGuard};
use super::HEAP_ALLOCATOR;
use ucore_memory::{*, paging::PageTable};
use ucore_memory::cow::CowExt;
pub use ucore_memory::memory_set::{MemoryArea, MemoryAttr, MemorySet as MemorySet_, Stack};

pub type MemorySet = MemorySet_<InactivePageTable0>;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<BitAlloc64K> = Mutex::new(BitAlloc64K::default());
}

pub fn alloc_frame() -> Option<usize> {
    let ret = FRAME_ALLOCATOR.lock().alloc().map(|id| id * PAGE_SIZE + MEMORY_OFFSET);
    trace!("Allocate frame: {:x?}", ret);
    ret
}

pub fn dealloc_frame(target: usize) {
    trace!("Deallocate frame: {:x}", target);
    FRAME_ALLOCATOR.lock().dealloc((target - MEMORY_OFFSET) / PAGE_SIZE);
}

// alloc from heap
pub fn alloc_stack() -> Stack {
    use alloc::alloc::{alloc, Layout};
    const STACK_SIZE: usize = 0x8000;
    let bottom = unsafe{ alloc(Layout::from_size_align(STACK_SIZE, 0x8000).unwrap()) } as usize;
    let top = bottom + STACK_SIZE;
    Stack { top, bottom }
}

lazy_static! {
    static ref ACTIVE_TABLE: Mutex<CowExt<ActivePageTable>> = Mutex::new(unsafe {
        CowExt::new(ActivePageTable::new())
    });
}

/// The only way to get active page table
pub fn active_table() -> MutexGuard<'static, CowExt<ActivePageTable>> {
    ACTIVE_TABLE.lock()
}

// Return true to continue, false to halt
pub fn page_fault_handler(addr: usize) -> bool {
    // Handle copy on write
    unsafe { ACTIVE_TABLE.force_unlock(); }
    active_table().page_fault_handler(addr, || alloc_frame().unwrap())
}

pub fn init_heap() {
    use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
    unsafe { HEAP_ALLOCATOR.lock().init(KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE); }
    info!("heap init end");
}

//pub mod test {
//    pub fn cow() {
//        use super::*;
//        use ucore_memory::cow::test::test_with;
//        test_with(&mut active_table());
//    }
//}