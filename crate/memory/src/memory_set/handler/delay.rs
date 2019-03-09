use super::*;

#[derive(Debug, Clone)]
pub struct Delay<T: FrameAllocator> {
    allocator: T,
}

impl<T: FrameAllocator> MemoryHandler for Delay<T> {
    fn box_clone(&self) -> Box<MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut PageTable, addr: VirtAddr, attr: &MemoryAttr) {
        let entry = pt.map(addr, 0);
        entry.set_present(false);
        attr.apply(entry);
    }

    fn map_eager(&self, pt: &mut PageTable, addr: VirtAddr, attr: &MemoryAttr) {
        let target = self.allocator.alloc().expect("failed to alloc frame");
        let entry = pt.map(addr, target);
        entry.set_present(true);
        attr.apply(entry);
    }

    fn unmap(&self, pt: &mut PageTable, addr: VirtAddr) {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            self.allocator.dealloc(entry.target());
        }

        // PageTable::unmap requires page to be present
        entry.set_present(true);
        pt.unmap(addr);
    }

    fn handle_page_fault(&self, pt: &mut PageTable, addr: VirtAddr) -> bool {
        let entry = pt.get_entry(addr).expect("failed to get entry");
        if entry.present() {
            // not a delay case
            return false;
        }
        let frame = self.allocator.alloc().expect("failed to alloc frame");
        entry.set_target(frame);
        entry.set_present(true);
        entry.update();
        true
    }
}

impl<T: FrameAllocator> Delay<T> {
    pub fn new(allocator: T) -> Self {
        Delay { allocator }
    }
}
