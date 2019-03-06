use super::*;

#[derive(Debug, Clone)]
pub struct Delay<T: FrameAllocator> {
    flags: MemoryAttr,
    allocator: T,
}

impl<T: FrameAllocator> MemoryHandler for Delay<T> {
    fn box_clone(&self) -> Box<MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut PageTable, addr: VirtAddr) {
        let entry = pt.map(addr, 0);
        entry.set_present(false);
        entry.update();
    }

    fn map_eager(&self, pt: &mut PageTable, addr: VirtAddr) {
        let target = self.allocator.alloc().expect("failed to alloc frame");
        self.flags.apply(pt.map(addr, target));
    }

    fn unmap(&self, pt: &mut PageTable, addr: VirtAddr) {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            self.allocator.dealloc(entry.target());
            pt.unmap(addr);
        }
    }

    fn page_fault_handler(&self, pt: &mut PageTable, addr: VirtAddr) -> bool {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            // not a delay case
            return false;
        }
        let frame = self.allocator.alloc().expect("failed to alloc frame");
        entry.set_target(frame);
        self.flags.apply(entry);
        true
    }
}

impl<T: FrameAllocator> Delay<T> {
    pub fn new(flags: MemoryAttr, allocator: T) -> Self {
        Delay { flags, allocator }
    }
}
