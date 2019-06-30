// Simple kernel memory set for kernel virtual memory
use crate::arch::paging::PageTableImpl;
use crate::consts::*;
use crate::memory::GlobalFrameAlloc;
use crate::sync::SpinLock as Mutex;
use alloc::vec::*;
use buddy_system_allocator::*;
use core::alloc::Layout;
use core::mem::ManuallyDrop;
use core::ops::DerefMut;
use core::ptr::NonNull;
use lazy_static::lazy_static;
use rcore_memory::memory_set::handler::{ByFrame, MemoryHandler};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::{Page, PAGE_SIZE};

///Allocated virtual memory space by pages. returns some vaddr.
pub trait MemorySpaceManager {
    fn new() -> Self;
    fn alloc(&mut self, size: usize) -> Option<(usize, usize)>;
    fn free(&mut self, target: (usize, usize));
    fn kernel_table(&self) -> ManuallyDrop<PageTableImpl> {
        // Only one process can change the kernel table at a time.
        // If you want to change the mapping item, you have to lock the MemorySpaceManager.
        unsafe { PageTableImpl::kernel_table() }
    }
}

/// The most simple strategy: no free and allocate ahead.
/// TODO: A better allocation strategy required.
pub struct LinearManager {
    last_page: usize,
}
use crate::arch::consts::KSEG2_START;

impl MemorySpaceManager for LinearManager {
    fn new() -> LinearManager {
        LinearManager { last_page: 0 }
    }
    fn alloc(&mut self, size: usize) -> Option<(usize, usize)> {
        let mut required_pages = (size + PAGE_SIZE - 1) / PAGE_SIZE;

        let current = self.last_page * PAGE_SIZE + KSEG2_START;
        self.last_page += required_pages;
        Some((current, required_pages * PAGE_SIZE))
    }

    fn free(&mut self, (addr, size): (usize, usize)) {
        //Do nothing.
    }
}

type VirtualMemorySpaceManager = LinearManager;
type LockedVMM = Mutex<VirtualMemorySpaceManager>;
lazy_static! {
    pub static ref KERNELVM_MANAGER: LockedVMM = Mutex::new(VirtualMemorySpaceManager::new());
}

/// Represents a contiguous virtual area: like the ancient const_reloc.
/// Use RAII for exception handling
pub struct VirtualSpace {
    start: usize,
    size: usize,
    areas: Vec<VirtualArea>,
    allocator: &'static LockedVMM,
    page_allocator: ByFrame<GlobalFrameAlloc>,
}

impl VirtualSpace {
    pub fn new(allocator: &'static LockedVMM, size: usize) -> Option<VirtualSpace> {
        let mut vmm = allocator.lock();
        let (start, rsize) = vmm.alloc(size)?;
        Some(VirtualSpace {
            start: start,
            size: rsize,
            areas: Vec::new(),
            allocator: allocator,
            page_allocator: ByFrame::new(GlobalFrameAlloc),
        })
    }
    pub fn start(&self) -> usize {
        self.start
    }
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn add_area(
        &mut self,
        start_addr: usize,
        end_addr: usize,
        attr: &MemoryAttr,
    ) -> &VirtualArea {
        let area = VirtualArea::new(start_addr, end_addr - start_addr, attr, self);
        self.areas.push(area);
        self.areas.last().unwrap()
    }
}

impl Drop for VirtualSpace {
    fn drop(&mut self) {
        for mut v in self.areas.iter_mut() {
            v.unmap(self.allocator, &mut self.page_allocator);
        }
    }
}

pub struct VirtualArea {
    start: usize,
    end: usize,
    attr: MemoryAttr,
}

impl VirtualArea {
    pub fn new(
        page_addr: usize,
        size: usize,
        attr: &MemoryAttr,
        parent: &mut VirtualSpace,
    ) -> VirtualArea {
        let aligned_start_addr = page_addr - page_addr % PAGE_SIZE;
        let mut aligned_end = (page_addr + size + PAGE_SIZE - 1);
        aligned_end = aligned_end - aligned_end % PAGE_SIZE;
        let lock = parent.allocator.lock();
        let mut active_pt = lock.kernel_table();
        for p in Page::range_of(aligned_start_addr, aligned_end) {
            parent
                .page_allocator
                .map(active_pt.deref_mut(), p.start_address(), attr);
        }

        VirtualArea {
            start: aligned_start_addr,
            end: aligned_end,
            attr: attr.clone(),
        }
    }
    pub fn unmap(&mut self, allocator: &LockedVMM, parent: &mut ByFrame<GlobalFrameAlloc>) {
        let lock = allocator.lock();
        let mut active_pt = lock.kernel_table();
        for p in Page::range_of(self.start, self.end) {
            parent.unmap(active_pt.deref_mut(), p.start_address());
        }
    }
}
