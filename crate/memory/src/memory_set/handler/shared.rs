use super::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use spin::Mutex;

// represent physical memory area
#[derive(Debug)]
pub struct SharedGuard<T: FrameAllocator> {
    allocator: T,
    pub size: usize,
    // indirect mapping now. target: page_offset -> physAddr
    target: BTreeMap<usize, usize>,
}

impl<T: FrameAllocator> SharedGuard<T> {
    pub fn new(allocator: T) -> Self {
        SharedGuard {
            allocator: allocator,
            // size meaningful only for sys_shm
            size: 0,
            target: BTreeMap::new(),
        }
    }

    pub fn new_with_size(allocator: T, size: usize) -> Self {
        SharedGuard {
            allocator: allocator,
            size: size,
            target: BTreeMap::new(),
        }
    }

    pub fn alloc(&mut self, virt_addr: usize) -> Option<usize> {
        let phys_addr = self.allocator.alloc().expect("failed to allocate frame");
        self.target.insert(virt_addr, phys_addr);
        Some(phys_addr)
    }

    pub fn dealloc(&mut self, virt_addr: usize) {
        let phys_addr = self.target.get(&virt_addr).unwrap().clone();
        self.allocator.dealloc(phys_addr);
        self.target.remove(&virt_addr);
    }

    pub fn get(&self, addr: usize) -> Option<usize> {
        match self.target.get(&addr) {
            Some(phys_addr) => Some(phys_addr.clone()),
            None => None,
        }
    }
}

impl<T: FrameAllocator> Drop for SharedGuard<T> {
    fn drop(&mut self) {
        let mut free_list = Vec::new();
        for (virt_addr, _phys_addr) in self.target.iter() {
            free_list.push(virt_addr.clone());
        }
        for virt_addr in free_list.iter() {
            self.dealloc(virt_addr.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct Shared<T: FrameAllocator> {
    allocator: T,
    // used as an indirection layer to hack rust "mut" protection
    start_virt_addr: Arc<Mutex<Option<usize>>>,
    guard: Arc<Mutex<SharedGuard<T>>>,
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
        if self.start_virt_addr.lock().is_none() {
            let mut init_start_virt_addr = self.start_virt_addr.lock();
            *init_start_virt_addr = Some(addr);
        }
        let addr_offset = addr - self.start_virt_addr.lock().unwrap();
        let phys_addr_opt = self.guard.lock().get(addr_offset);
        if phys_addr_opt.is_none() {
            // not mapped yet
            let entry = pt.map(addr, 0);
            entry.set_present(false);
            attr.apply(entry);
        } else {
            // physical memory already allocated by other process
            let phys_addr = phys_addr_opt.unwrap().clone();
            let entry = pt.map(addr, phys_addr);
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
        let addr_offset = addr - self.start_virt_addr.lock().unwrap();
        let phys_addr_opt = self.guard.lock().get(addr_offset);
        if entry.present() {
            // not a delay case
            return false;
        } else if phys_addr_opt.is_none() {
            // physical memory not alloced.
            let frame = self.guard.lock().alloc(addr_offset).unwrap();
            entry.set_target(frame);
            entry.set_present(true);
            entry.update();

            // init with zero for delay mmap mode
            let data = pt.get_page_slice_mut(addr);
            let len = data.len();
            for x in data {
                *x = 0;
            }
            pt.flush_cache_copy_user(addr, addr + len, false);
        } else {
            // physical memory alloced. update page table
            let frame = phys_addr_opt.unwrap().clone();
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
            start_virt_addr: Arc::new(Mutex::new(None)),
            guard: Arc::new(Mutex::new(SharedGuard::new(allocator))),
        }
    }

    pub fn new_with_guard(allocator: T, guard: Arc<Mutex<SharedGuard<T>>>) -> Self {
        Shared {
            allocator: allocator.clone(),
            start_virt_addr: Arc::new(Mutex::new(None)),
            guard: guard.clone(),
        }
    }
}
