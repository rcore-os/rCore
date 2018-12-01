pub use crate::arch::paging::*;
use bit_allocator::{BitAlloc, BitAlloc4K, BitAlloc64K, BitAlloc1M};
use crate::consts::MEMORY_OFFSET;
use super::HEAP_ALLOCATOR;
use ucore_memory::{*, paging::PageTable};
use ucore_memory::cow::CowExt;
pub use ucore_memory::memory_set::{MemoryArea, MemoryAttr, InactivePageTable};
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

// Page table for swap in and out
lazy_static!{
    static ref ACTIVE_TABLE_SWAP: SpinNoIrqLock<SwapExt<ActivePageTable, fifo::FifoSwapManager, mock_swapper::MockSwapper>> =
        SpinNoIrqLock::new(unsafe{SwapExt::new(ActivePageTable::new(), fifo::FifoSwapManager::default(), mock_swapper::MockSwapper::default())});
}

pub fn active_table_swap() -> MutexGuard<'static, SwapExt<ActivePageTable, fifo::FifoSwapManager, mock_swapper::MockSwapper>, SpinNoIrq>{
    ACTIVE_TABLE_SWAP.lock()
}

/*
* @brief:
*   allocate a free physical frame, if no free frame, then swap out one page and reture mapped frame as the free one
* @retval:
*   the physical address for the allocated frame
*/
pub fn alloc_frame() -> Option<usize> {
    // get the real address of the alloc frame
    let ret = FRAME_ALLOCATOR.lock().alloc().map(|id| id * PAGE_SIZE + MEMORY_OFFSET);
    trace!("Allocate frame: {:x?}", ret);
    //do we need : unsafe { ACTIVE_TABLE_SWAP.force_unlock(); } ???
    Some(ret.unwrap_or_else(|| active_table_swap().swap_out_any::<InactivePageTable0>().ok().expect("fail to swap out page")))
}

pub fn dealloc_frame(target: usize) {
    trace!("Deallocate frame: {:x}", target);
    FRAME_ALLOCATOR.lock().dealloc((target - MEMORY_OFFSET) / PAGE_SIZE);
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


/*
* @param:
*   addr: the virtual address of the page fault
* @brief:
*   handle page fault
* @retval:
*   Return true to continue, false to halt
*/
#[cfg(not(feature = "no_mmu"))]
pub fn page_fault_handler(addr: usize) -> bool {
    info!("start handling swap in/out page fault");
    //unsafe { ACTIVE_TABLE_SWAP.force_unlock(); }

    info!("active page table token in pg fault is {:x?}", ActivePageTable::token());
    /*LAB3 EXERCISE 1: YOUR STUDENT NUMBER
    * handle the frame deallocated
    */

    info!("get pt from processor()");
    if process().memory_set.find_area(addr).is_none(){
        return false;
    }

    let pt = process().memory_set.get_page_table_mut();
    info!("pt got");
    if active_table_swap().page_fault_handler(pt as *mut InactivePageTable0, addr, true, || alloc_frame().expect("fail to alloc frame")){
        return true;
    }
    //////////////////////////////////////////////////////////////////////////////


    // Handle copy on write (not being used now)
    /*
    unsafe { ACTIVE_TABLE.force_unlock(); }
    if active_table().page_fault_handler(addr, || alloc_frame().expect("fail to alloc frame")){
        return true;
    }
    */
    false
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
