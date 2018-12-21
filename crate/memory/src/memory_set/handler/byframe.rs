use super::*;

#[derive(Debug, Clone)]
pub struct ByFrame<T: FrameAllocator> {
    flags: MemoryAttr,
    allocator: T,
}

impl<T: FrameAllocator> MemoryHandler for ByFrame<T> {
    fn box_clone(&self) -> Box<MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut PageTable, addr: VirtAddr) {
        let target = self.allocator.alloc().expect("failed to allocate frame");
        self.flags.apply(pt.map(addr, target));
    }

    fn unmap(&self, pt: &mut PageTable, addr: VirtAddr) {
        let target = pt.get_entry(addr).expect("fail to get entry").target();
        self.allocator.dealloc(target);
        pt.unmap(addr);
    }

    fn page_fault_handler(&self, _pt: &mut PageTable, _addr: VirtAddr) -> bool {
        false
    }
}

impl<T: FrameAllocator> ByFrame<T> {
    pub fn new(flags: MemoryAttr, allocator: T) -> Self {
        ByFrame { flags, allocator }
    }
}