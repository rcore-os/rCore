use super::*;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use spin::Mutex;

// represent physical memory area
#[derive(Debug)]
pub struct SharedGuard<T: FrameAllocator> {
    allocator: T,
    pub size: usize,
    // indirect mapping now. target: page_offset -> physAddr
    target: BTreeMap<usize, usize> 
}

impl<T: FrameAllocator> SharedGuard<T> {
    pub fn new(allocator: T) -> Self {
        SharedGuard { 
            allocator: allocator,
            size: 0,
            target: BTreeMap::new()
        } 
        // size meaningful only for sys_shm
    }
    pub fn new_with_size(allocator: T, size: usize) -> Self {
        SharedGuard {
            allocator: allocator,
            size: size,
            target: BTreeMap::new()
        }
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
        match self.target.get(&addr) {
            Some(physAddr) => Some(physAddr.clone()),
            None => None
        }
        //Some(self.target.get(&addr).unwrap().clone())
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
    // used as an indirection layer to hack rust "mut" protection
    startVirtAddr: Arc<Mutex<Option<usize>>>,
    guard: Arc<Mutex<SharedGuard<T>>>
}

impl<T: FrameAllocator> MemoryHandler for Shared<T> {
    fn box_clone(&self) -> Box<dyn MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut dyn PageTable, addr: VirtAddr, attr: &MemoryAttr) {
        //assert!(self.guard.is_some(), "remapping memory area")
        // you have to make sure that this function is called in a sequential order
        // I assume that the first call of this function pass the startVirtualAddr of the MemoryArea
        // TODO: Remove this potential bug
        // Remember that pages can be randomly delayed allocated by all sharing threads
        // Take care when to use "addrOffset" instead of "addr"
        // Hack
        if self.startVirtAddr.lock().is_none() {
            let mut initStartVirtAddr = self.startVirtAddr.lock();
            *initStartVirtAddr = Some(addr);
        }
        let addrOffset = addr - self.startVirtAddr.lock().unwrap();
        let physAddrOpt = self.guard.lock().get(addrOffset);
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
        let addrOffset = addr - self.startVirtAddr.lock().unwrap();
        let physAddrOpt = self.guard.lock().get(addrOffset);
        if entry.present() {
            // not a delay case
            return false;
        } else if physAddrOpt.is_none() {
            // physical memory not alloced.
            let frame = self.guard.lock().alloc(addrOffset).unwrap();
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
            startVirtAddr: Arc::new(Mutex::new(None)),
            guard: Arc::new(Mutex::new(SharedGuard::new(allocator)))
        }
    }
    pub fn new_with_guard(allocator: T, guard: Arc<Mutex<SharedGuard<T>>>) -> Self {
        Shared {
            allocator: allocator.clone(),
            startVirtAddr: Arc::new(Mutex::new(None)),
            guard: guard.clone()
        }
    }
}