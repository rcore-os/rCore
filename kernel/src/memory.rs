pub use arch::paging::*;
use bit_allocator::{BitAlloc, BitAlloc4K, BitAlloc64K};
use consts::MEMORY_OFFSET;
use spin::{Mutex, MutexGuard};
use super::HEAP_ALLOCATOR;
use ucore_memory::{*, paging::PageTable};
use ucore_memory::cow::CowExt;
pub use ucore_memory::memory_set::{MemoryArea, MemoryAttr, MemorySet as MemorySet_, Stack};
use ucore_memory::swap::*;
use alloc::collections::VecDeque;
use process::processor;

pub type MemorySet = MemorySet_<InactivePageTable0>;

// x86_64 support up to 256M memory
#[cfg(target_arch = "x86_64")]
pub type FrameAlloc = BitAlloc64K;

// RISCV only have 8M memory
#[cfg(target_arch = "riscv32")]
pub type FrameAlloc = BitAlloc4K;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAlloc> = Mutex::new(FrameAlloc::default());
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

// Page table for swap in and out
lazy_static!{
    static ref ACTIVE_TABLE_SWAP: Mutex<SwapExt<ActivePageTable, fifo::FifoSwapManager, mock_swapper::MockSwapper>> = 
        Mutex::new(unsafe{SwapExt::new(ActivePageTable::new(), fifo::FifoSwapManager::default(), mock_swapper::MockSwapper::default())});
}

pub fn active_table_swap() -> MutexGuard<'static, SwapExt<ActivePageTable, fifo::FifoSwapManager, mock_swapper::MockSwapper>>{
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
    Some(ret.unwrap_or_else(|| active_table_swap().swap_out_any::<InactivePageTable0>().ok().unwrap()))
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



/* 
* @param:
*   addr: the virtual address of the page fault
* @brief: 
*   handle page fault
* @retval: 
*   Return true to continue, false to halt  
*/
pub fn page_fault_handler(addr: usize) -> bool {
    // Handle copy on write (not being used now)
    unsafe { ACTIVE_TABLE.force_unlock(); }
    if active_table().page_fault_handler(addr, || alloc_frame().unwrap()){
        return true;
    }
    // handle the swap in/out
    info!("start handling swap in/out page fault");
    unsafe { ACTIVE_TABLE_SWAP.force_unlock(); }
    let mut temp_proc = processor();
    let pt = temp_proc.current_context_mut().get_memory_set_mut().get_page_table_mut();
    if active_table_swap().page_fault_handler(pt as *mut InactivePageTable0, addr, || alloc_frame().unwrap()){
        return true;
    }
    false
}

pub fn init_heap() {
    use consts::KERNEL_HEAP_SIZE;
    static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
    unsafe { HEAP_ALLOCATOR.lock().init(HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE); }
    info!("heap init end");
}

//pub mod test {
//    pub fn cow() {
//        use super::*;
//        use ucore_memory::cow::test::test_with;
//        test_with(&mut active_table());
//    }
//}