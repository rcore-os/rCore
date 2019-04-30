//! Define the FrameAllocator for physical memory
//! x86_64      --  64GB
//! AARCH64/MIPS/RV --  1GB
//! K210(rv64)  --  8MB
//! NOTICE:
//! type FrameAlloc = bitmap_allocator::BitAllocXXX
//! KSTACK_SIZE         -- 16KB
//!
//! KERNEL_HEAP_SIZE:
//! x86-64              -- 32MB
//! AARCH64/RV64        -- 8MB
//! MIPS/RV32           -- 2MB
//! mipssim/malta(MIPS) -- 10MB

use super::HEAP_ALLOCATOR;
pub use crate::arch::paging::*;
use crate::consts::MEMORY_OFFSET;
use crate::process::process_unsafe;
use crate::sync::SpinNoIrqLock;
use bitmap_allocator::BitAlloc;
use buddy_system_allocator::LockedHeap;
use lazy_static::*;
use log::*;
pub use rcore_memory::memory_set::{handler::*, MemoryArea, MemoryAttr};
use rcore_memory::*;

pub type MemorySet = rcore_memory::memory_set::MemorySet<InactivePageTable0>;

// x86_64 support up to 64G memory
#[cfg(target_arch = "x86_64")]
pub type FrameAlloc = bitmap_allocator::BitAlloc16M;

// RISCV, ARM, MIPS has 1G memory
#[cfg(all(
    any(
        target_arch = "riscv32",
        target_arch = "riscv64",
        target_arch = "aarch64",
        target_arch = "mips"
    ),
    not(feature = "board_k210")
))]
pub type FrameAlloc = bitmap_allocator::BitAlloc1M;

// K210 has 8M memory
#[cfg(feature = "board_k210")]
pub type FrameAlloc = bitmap_allocator::BitAlloc4K;

lazy_static! {
    pub static ref FRAME_ALLOCATOR: SpinNoIrqLock<FrameAlloc> =
        SpinNoIrqLock::new(FrameAlloc::default());
}

/// The only way to get active page table
///
/// ## CHANGE LOG
///
/// In the past, this function returns a `MutexGuard` of a global
/// `Mutex<ActiveTable>` object, which means only one CPU core
/// can access its active table at a time.
///
/// But given that a page table is ** process local **, and being active
/// when and only when a thread of the process is running.
/// The ownership of this page table is in the `MemorySet` object.
/// So it's safe to access the active table inside `MemorySet`.
/// But the shared parts is readonly, e.g. all pages mapped in
/// `InactivePageTable::map_kernel()`.
pub fn active_table() -> ActivePageTable {
    unsafe { ActivePageTable::new() }
}

#[derive(Debug, Clone, Copy)]
pub struct GlobalFrameAlloc;

impl FrameAllocator for GlobalFrameAlloc {
    fn alloc(&self) -> Option<usize> {
        // get the real address of the alloc frame
        let ret = FRAME_ALLOCATOR
            .lock()
            .alloc()
            .map(|id| id * PAGE_SIZE + MEMORY_OFFSET);
        trace!("Allocate frame: {:x?}", ret);
        ret
        // TODO: try to swap out when alloc failed
    }
    fn dealloc(&self, target: usize) {
        trace!("Deallocate frame: {:x}", target);
        FRAME_ALLOCATOR
            .lock()
            .dealloc((target - MEMORY_OFFSET) / PAGE_SIZE);
    }
}

pub fn alloc_frame() -> Option<usize> {
    GlobalFrameAlloc.alloc()
}
pub fn dealloc_frame(target: usize) {
    GlobalFrameAlloc.dealloc(target);
}

pub struct KernelStack(usize);
const KSTACK_SIZE: usize = 0x4000; //16KB

impl KernelStack {
    pub fn new() -> Self {
        use alloc::alloc::{alloc, Layout};
        let bottom =
            unsafe { alloc(Layout::from_size_align(KSTACK_SIZE, KSTACK_SIZE).unwrap()) } as usize;
        KernelStack(bottom)
    }
    pub fn top(&self) -> usize {
        self.0 + KSTACK_SIZE
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        use alloc::alloc::{dealloc, Layout};
        unsafe {
            dealloc(
                self.0 as _,
                Layout::from_size_align(KSTACK_SIZE, KSTACK_SIZE).unwrap(),
            );
        }
    }
}

/// Handle page fault at `addr`.
/// Return true to continue, false to halt.
pub fn handle_page_fault(addr: usize) -> bool {
    debug!("page fault @ {:#x}", addr);

    // This is safe as long as page fault never happens in page fault handler
    unsafe { process_unsafe().vm.handle_page_fault(addr) }
}

pub fn init_heap() {
    use crate::consts::KERNEL_HEAP_SIZE;
    static mut HEAP: [u8; KERNEL_HEAP_SIZE] = [0; KERNEL_HEAP_SIZE];
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, KERNEL_HEAP_SIZE);
    }
    info!("heap init end");
}

/// Allocator for the rest memory space on NO-MMU case.
pub static MEMORY_ALLOCATOR: LockedHeap = LockedHeap::empty();
