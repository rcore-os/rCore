use super::*;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Linear {
    offset: isize,
    flags: MemoryAttr,
}

impl MemoryHandler for Linear {
    fn box_clone(&self) -> Box<MemoryHandler> {
        Box::new(self.clone())
    }

    fn map(&self, pt: &mut PageTable, addr: VirtAddr) {
        let target = (addr as isize + self.offset) as PhysAddr;
        self.flags.apply(pt.map(addr, target));
    }

    fn unmap(&self, pt: &mut PageTable, addr: VirtAddr) {
        pt.unmap(addr);
    }

    fn page_fault_handler(&self, _pt: &mut PageTable, _addr: VirtAddr) -> bool {
        false
    }
}

impl Linear {
    pub fn new(offset: isize, flags: MemoryAttr) -> Self {
        Linear { offset, flags }
    }
}