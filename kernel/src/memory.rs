//! Define the FrameAllocator for physical memory

use super::HEAP_ALLOCATOR;
use crate::consts::{KERNEL_OFFSET, MEMORY_OFFSET, PHYSICAL_MEMORY_OFFSET};
use crate::process::current_thread;
use crate::sync::SpinNoIrqLock;
use bitmap_allocator::BitAlloc;
use buddy_system_allocator::Heap;
use core::mem;
use core::mem::size_of;
use log::*;
use rcore_memory::*;

pub use crate::arch::paging::*;
pub use rcore_memory::memory_set::{handler::*, MemoryArea, MemoryAttr};
pub type MemorySet = rcore_memory::memory_set::MemorySet<PageTableImpl>;

// x86_64 support up to 1T memory
#[cfg(target_arch = "x86_64")]
pub type FrameAlloc = bitmap_allocator::BitAlloc256M;

// RISCV, ARM, MIPS has 1G memory
#[cfg(any(
    target_arch = "riscv32",
    target_arch = "riscv64",
    target_arch = "aarch64",
    target_arch = "mips"
))]
pub type FrameAlloc = bitmap_allocator::BitAlloc1M;

pub static FRAME_ALLOCATOR: SpinNoIrqLock<FrameAlloc> = SpinNoIrqLock::new(FrameAlloc::DEFAULT);

/// Convert physical address to virtual address
#[inline]
#[cfg(not(mipsel))]
pub const fn phys_to_virt(paddr: usize) -> usize {
    PHYSICAL_MEMORY_OFFSET + paddr
}

/// MIPS is special
#[inline]
#[cfg(mipsel)]
pub const fn phys_to_virt(paddr: usize) -> usize {
    if paddr <= PHYSICAL_MEMORY_OFFSET {
        PHYSICAL_MEMORY_OFFSET + paddr
    } else {
        paddr
    }
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
    fn alloc_contiguous(&self, size: usize, align_log2: usize) -> Option<PhysAddr> {
        // get the real address of the alloc frame
        let ret = FRAME_ALLOCATOR
            .lock()
            .alloc_contiguous(size, align_log2)
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
pub fn alloc_frame_contiguous(size: usize, align_log2: usize) -> Option<usize> {
    GlobalFrameAlloc.alloc_contiguous(size, align_log2)
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
    debug!("page fault from kernel @ {:#x}", addr);

    let thread = current_thread().unwrap();
    let mut lock = thread.vm.lock();
    lock.handle_page_fault(addr)
}

/// Handle page fault at `addr` with access type `access`.
/// Return true to continue, false to halt.
pub fn handle_page_fault_ext(addr: usize, access: crate::memory::AccessType) -> bool {
    debug!(
        "page fault from kernel @ {:#x} with access type {:?}",
        addr, access
    );

    let thread = current_thread().unwrap();
    let mut lock = thread.vm.lock();
    lock.handle_page_fault_ext(addr, access)
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
}

pub fn enlarge_heap(heap: &mut Heap) {
    info!("Enlarging heap to avoid oom");

    let mut addrs = [(0, 0); 32];
    let mut addr_len = 0;
    let va_offset = PHYSICAL_MEMORY_OFFSET;
    for _ in 0..16384 {
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

pub fn copy_to_user<T>(addr: *mut T, src: *const T) -> bool {
    #[inline(never)]
    #[link_section = ".text.copy_user"]
    unsafe extern "C" fn write_user<T>(dst: *mut T, src: *const T) -> usize {
        dst.copy_from_nonoverlapping(src, 1);
        0
    }
    if !access_ok(addr as usize, size_of::<T>()) {
        return false;
    }
    match unsafe { write_user(addr, src) } {
        0 => true,
        _ => false,
    }
}
