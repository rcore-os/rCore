//! Used for delay mapping host's virtual memory to guest's physical memory

use alloc::{boxed::Box, sync::Arc};

use rvm::RvmPageTable;
use rvm::{DefaultGuestPhysMemorySet, GuestMemoryAttr, GuestPhysAddr, HostVirtAddr};

use rcore_memory::memory_set::handler::{FrameAllocator, MemoryHandler};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::paging::PageTable;

#[derive(Debug, Clone)]
pub struct RvmPageTableHandlerDelay<T: FrameAllocator> {
    guest_start_paddr: GuestPhysAddr,
    host_start_vaddr: HostVirtAddr,
    gpm: Arc<DefaultGuestPhysMemorySet>,
    allocator: T,
}

impl<T: FrameAllocator> RvmPageTableHandlerDelay<T> {
    pub fn new(
        guest_start_paddr: GuestPhysAddr,
        host_start_vaddr: HostVirtAddr,
        gpm: Arc<DefaultGuestPhysMemorySet>,
        allocator: T,
    ) -> Self {
        Self {
            guest_start_paddr,
            host_start_vaddr,
            gpm,
            allocator,
        }
    }
}

impl<T: FrameAllocator> MemoryHandler for RvmPageTableHandlerDelay<T> {
    fn box_clone(&self) -> Box<dyn MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut dyn PageTable, addr: HostVirtAddr, attr: &MemoryAttr) {
        let entry = pt.map(addr, 0);
        entry.set_present(false);
        attr.apply(entry);
    }

    fn unmap(&self, pt: &mut dyn PageTable, addr: HostVirtAddr) {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        // PageTable::unmap requires page to be present
        entry.set_present(true);
        pt.unmap(addr);
    }

    fn clone_map(
        &self,
        pt: &mut dyn PageTable,
        src_pt: &mut dyn PageTable,
        addr: HostVirtAddr,
        attr: &MemoryAttr,
    ) {
        let entry = src_pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            // eager map and copy data
            let data = src_pt.get_page_slice_mut(addr);
            let target = self.allocator.alloc().expect("failed to alloc frame");
            let entry = pt.map(addr, target);
            attr.apply(entry);
            pt.get_page_slice_mut(addr).copy_from_slice(data);
        } else {
            // delay map
            self.map(pt, addr, attr);
        }
    }

    fn handle_page_fault(&self, pt: &mut dyn PageTable, addr: HostVirtAddr) -> bool {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            // not a delay case
            return false;
        }

        let guest_paddr = addr - self.host_start_vaddr + self.guest_start_paddr;
        let mut rvm_pt = self.gpm.rvm_page_table.lock();
        let mut target = rvm_pt.query(guest_paddr).unwrap_or(0);
        if target == 0 {
            target = self.allocator.alloc().expect("failed to alloc frame");
        }
        rvm_pt
            .map(guest_paddr, target, GuestMemoryAttr::default())
            .expect("failed to create GPA -> HPA mapping");

        entry.set_target(target);
        entry.set_present(true);
        entry.update();
        true
    }
}
