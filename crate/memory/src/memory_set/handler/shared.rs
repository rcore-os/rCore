use super::*;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use spin::Mutex;

#[derive(Debug)]
struct SharedGuard<T: FrameAllocator> {
    allocator: T,
    // direct mapping now. only work for mmap
    target: BTreeMap<usize, usize> 
}

impl<T: FrameAllocator> SharedGuard<T> {
    pub fn new(allocator: T) -> Self {
        SharedGuard { 
            allocator: allocator,
            target: BTreeMap::new()
        } // delayed allocated now
    }
    pub fn alloc(&mut self, virtAddr: usize) -> Option<usize> {
        let physAddr = self.allocator.alloc().expect("failed to allocate frame");
        self.target.insert(virtAddr, physAddr);
        Some(physAddr)
    }
    pub fn dealloc(&mut self, virtAddr: usize) {
        let physAddr = self.target.get(&virtAddr).unwrap().clone();
        self.allocator.dealloc(physAddr);
        self.target.remove(&virtAddr);
    }
    pub fn get(&self, addr: usize) -> Option<usize> {
        Some(self.target.get(&addr).unwrap().clone())
    }
}

impl<T: FrameAllocator> Drop for SharedGuard<T> {
    fn drop(&mut self) {
        let mut freeList = Vec::new();
        for (virtAddr, _physAddr) in self.target.iter() {
            freeList.push(virtAddr.clone());
        }
        for virtAddr in freeList.iter() {
            self.dealloc(virtAddr.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shared<T: FrameAllocator> {
    allocator: T,
    guard: Option<Arc<Mutex<SharedGuard<T>>>>
}

impl<T: FrameAllocator> MemoryHandler for Shared<T> {
    fn box_clone(&self) -> Box<dyn MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut dyn PageTable, addr: VirtAddr, attr: &MemoryAttr) {
        //assert!(self.guard.is_some(), "remapping memory area")
        let guard = self.guard.clone();
        let physAddrOpt = guard.unwrap().lock().get(addr);
        if physAddrOpt.is_none() { // not mapped yet
            let entry = pt.map(addr, 0);
            entry.set_present(false);
            attr.apply(entry);
        } else  { // physical memory already allocated by other process
            let physAddr = physAddrOpt.unwrap().clone();
            let entry = pt.map(addr, physAddr);
            attr.apply(entry)
        }
    }

    fn unmap(&self, pt: &mut dyn PageTable, addr: VirtAddr) {
        // free physical memory done when guard destroyed
        pt.unmap(addr);
    }

    fn clone_map(
        &self,
        pt: &mut dyn PageTable,
        _src_pt: &mut dyn PageTable,
        addr: VirtAddr,
        attr: &MemoryAttr,
    ) {
        // actual map done when handling page fault, since guard are copied.
        let entry = pt.map(addr, 0);
        entry.set_present(false);
        attr.apply(entry);
    }

    fn handle_page_fault(&self, pt: &mut dyn PageTable, addr: VirtAddr) -> bool {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        let guard = self.guard.clone();
        let physAddrOpt = guard.clone().unwrap().lock().get(addr);
        if entry.present() {
            // not a delay case
            return false;
        } else if physAddrOpt.is_none() {
            // physical memory not alloced.
            let frame = guard.clone().unwrap().lock().alloc(addr).unwrap();
            entry.set_target(frame);
            entry.set_present(true);
            entry.update();

            //init with zero for delay mmap mode
            let data = pt.get_page_slice_mut(addr);
            let len = data.len();
            for x in data {
                *x = 0;
            }
            pt.flush_cache_copy_user(addr, addr + len, false);
        } else {
            // physical memory alloced. update page table
            let frame = physAddrOpt.unwrap().clone();
            entry.set_target(frame);
            entry.set_present(true);
            entry.update();
        }
        true
    }
}

impl<T: FrameAllocator> Shared<T> {
    pub fn new(allocator: T) -> Self {
        Shared {
            allocator: allocator.clone(),
            guard: Some(Arc::new(Mutex::new(SharedGuard::new(allocator))))
        }
    }
}