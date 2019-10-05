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
use crate::consts::{KERNEL_OFFSET, MEMORY_OFFSET, PHYSICAL_MEMORY_OFFSET};
use crate::process::current_thread;
use crate::sync::SpinNoIrqLock;
use bitmap_allocator::BitAlloc;
use buddy_system_allocator::Heap;
use core::mem;
use core::mem::size_of;
use lazy_static::*;
use log::*;
pub use rcore_memory::memory_set::{handler::*, MemoryArea, MemoryAttr};
use rcore_memory::*;

pub type MemorySet = rcore_memory::memory_set::MemorySet<PageTableImpl>;

// x86_64 support up to 1T memory
#[cfg(target_arch = "x86_64")]
pub type FrameAlloc = bitmap_allocator::BitAlloc256M;

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

/// Convert physical address to virtual address
#[inline]
pub const fn phys_to_virt(paddr: usize) -> usize {
    PHYSICAL_MEMORY_OFFSET + paddr
}

/// Convert virtual address to physical address
#[inline]
pub const fn virt_to_phys(vaddr: usize) -> usize {
    vaddr - PHYSICAL_MEMORY_OFFSET
}

/// Convert virtual address to the offset of kernel
#[inline]
pub const fn kernel_offset(vaddr: usize) -> usize {
    vaddr - KERNEL_OFFSET
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

    let thread = unsafe { current_thread() };
    thread.vm.lock().handle_page_fault(addr)
}

pub fn init_heap() {
    use crate::consts::KERNEL_HEAP_SIZE;
    const MACHINE_ALIGN: usize = mem::size_of::<usize>();
    const HEAP_BLOCK: usize = KERNEL_HEAP_SIZE / MACHINE_ALIGN;
    static mut HEAP: [usize; HEAP_BLOCK] = [0; HEAP_BLOCK];
    unsafe {
        HEAP_ALLOCATOR
            .lock()
            .init(HEAP.as_ptr() as usize, HEAP_BLOCK * MACHINE_ALIGN);
    }
    info!("heap init end");
}

pub fn enlarge_heap(heap: &mut Heap) {
    info!("Enlarging heap to avoid oom");

    let mut addrs = [(0, 0); 32];
    let mut addr_len = 0;
    let va_offset = PHYSICAL_MEMORY_OFFSET;
    for i in 0..16384 {
        let page = alloc_frame().unwrap();
        let va = va_offset + page;
        if addr_len > 0 {
            let (ref mut addr, ref mut len) = addrs[addr_len - 1];
            if *addr - PAGE_SIZE == va {
                *len += PAGE_SIZE;
                *addr -= PAGE_SIZE;
                continue;
            }
        }
        addrs[addr_len] = (va, PAGE_SIZE);
        addr_len += 1;
    }
    for (addr, len) in addrs[..addr_len].into_iter() {
        info!("Adding {:#X} {:#X} to heap", addr, len);
        unsafe {
            heap.init(*addr, *len);
        }
    }
}

/// Check whether the address range [addr, addr + len) is not in kernel space
pub fn access_ok(addr: usize, len: usize) -> bool {
    addr < PHYSICAL_MEMORY_OFFSET && (addr + len) < PHYSICAL_MEMORY_OFFSET
}

#[naked]
pub unsafe extern "C" fn read_user_fixup() -> usize {
    return 1;
}

pub fn copy_from_user<T>(addr: *const T) -> Option<T> {
    #[naked]
    #[inline(never)]
    #[link_section = ".text.copy_user"]
    unsafe extern "C" fn read_user<T>(dst: *mut T, src: *const T) -> usize {
        dst.copy_from_nonoverlapping(src, 1);
        0
    }
    if !access_ok(addr as usize, size_of::<T>()) {
        return None;
    }
    let mut dst: T = unsafe { core::mem::zeroed() };
    match unsafe { read_user(&mut dst, addr) } {
        0 => Some(dst),
        _ => None,
    }
}
