pub use crate::arch::paging::*;
use bit_allocator::{BitAlloc, BitAlloc4K, BitAlloc64K, BitAlloc1M};
use crate::consts::MEMORY_OFFSET;
use super::HEAP_ALLOCATOR;
use ucore_memory::{*, paging::PageTable};
use ucore_memory::cow::CowExt;
pub use ucore_memory::memory_set::{MemoryArea, MemoryAttr, handler::*};
use ucore_memory::swap::*;
use crate::process::{process};
use crate::sync::{SpinNoIrqLock, SpinNoIrq, MutexGuard};
use alloc::collections::VecDeque;
use lazy_static::*;
use log::*;
use linked_list_allocator::LockedHeap;

#[cfg(not(feature = "no_mmu"))]
pub type MemorySet = ucore_memory::memory_set::MemorySet<InactivePageTable0>;

#[cfg(feature = "no_mmu")]
pub type MemorySet = ucore_memory::no_mmu::MemorySet<NoMMUSupportImpl>;

// x86_64 support up to 256M memory
#[cfg(target_arch = "x86_64")]
pub type FrameAlloc = BitAlloc64K;

// RISCV only have 8M memory
#[cfg(target_arch = "riscv32")]
pub type FrameAlloc = BitAlloc4K;

// Raspberry Pi 3 has 1G memory
#[cfg(target_arch = "aarch64")]
pub type FrameAlloc = BitAlloc1M;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: SpinNoIrqLock<FrameAlloc> = SpinNoIrqLock::new(FrameAlloc::default());
}

lazy_static! {
    static ref ACTIVE_TABLE: SpinNoIrqLock<CowExt<ActivePageTable>> = SpinNoIrqLock::new(unsafe {
        CowExt::new(ActivePageTable::new())
    });
}

/// The only way to get active page table
pub fn active_table() -> MutexGuard<'static, CowExt<ActivePageTable>, SpinNoIrq> {
    ACTIVE_TABLE.lock()
}


#[derive(Debug, Clone, Copy)]
pub struct GlobalFrameAlloc;

impl FrameAllocator for GlobalFrameAlloc {
    fn alloc(&self) -> Option<usize> {
        // get the real address of the alloc frame
        let ret = FRAME_ALLOCATOR.lock().alloc().map(|id| id * PAGE_SIZE + MEMORY_OFFSET);
        trace!("Allocate frame: {:x?}", ret);
        ret
        // TODO: try to swap out when alloc failed
    }
    fn dealloc(&self, target: usize) {
        trace!("Deallocate frame: {:x}", target);
        FRAME_ALLOCATOR.lock().dealloc((target - MEMORY_OFFSET) / PAGE_SIZE);
    }
}

pub fn alloc_frame() -> Option<usize> {
    GlobalFrameAlloc.alloc()
}
pub fn dealloc_frame(target: usize) {
    GlobalFrameAlloc.dealloc(target);
}

pub struct KernelStack(usize);
const STACK_SIZE: usize = 0x8000;

impl KernelStack {
    pub fn new() -> Self {
        use alloc::alloc::{alloc, Layout};
        let bottom = unsafe{ alloc(Layout::from_size_align(STACK_SIZE, STACK_SIZE).unwrap()) } as usize;
        KernelStack(bottom)
    }
    pub fn top(&self) -> usize {
        self.0 + STACK_SIZE
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        use alloc::alloc::{dealloc, Layout};
        unsafe{ dealloc(self.0 as _, Layout::from_size_align(STACK_SIZE, STACK_SIZE).unwrap()); }
    }
}


/// Handle page fault at `addr`.
/// Return true to continue, false to halt.
#[cfg(not(feature = "no_mmu"))]
pub fn page_fault_handler(addr: usize) -> bool {
    info!("start handling swap in/out page fault");
    process().memory_set.page_fault_handler(addr)
}

pub fn init_heap() {
    use crate::consts::KERNEL_HEAP_SIZE;
    static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
    unsafe { HEAP_ALLOCATOR.lock().init(HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE); }
    info!("heap init end");
}

/// Allocator for the rest memory space on NO-MMU case.
pub static MEMORY_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[derive(Debug, Clone, Copy)]
pub struct NoMMUSupportImpl;

impl ucore_memory::no_mmu::NoMMUSupport for NoMMUSupportImpl {
    type Alloc = LockedHeap;
    fn allocator() -> &'static Self::Alloc {
        &MEMORY_ALLOCATOR
    }
}

#[cfg(feature = "no_mmu")]
pub fn page_fault_handler(_addr: usize) -> bool {
    unreachable!()
}


//pub mod test {
//    pub fn cow() {
//        use super::*;
//        use ucore_memory::cow::test::test_with;
//        test_with(&mut active_table());
//    }
//}
