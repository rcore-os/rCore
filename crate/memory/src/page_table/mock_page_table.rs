use alloc::btree_set::BTreeSet;
use super::*;

pub struct MockPageTable {
    mapped_set: BTreeSet<VirtAddr>,
    accessed_set: BTreeSet<VirtAddr>,
    dirty_set: BTreeSet<VirtAddr>,
    page_fault_handler: PageFaultHandler,
    capacity: usize,
}

type PageFaultHandler = fn(&mut MockPageTable, VirtAddr);

impl PageTable for MockPageTable {
    fn accessed(&self, addr: VirtAddr) -> bool {
        self.accessed_set.contains(&addr)
    }
    fn dirty(&self, addr: VirtAddr) -> bool {
        self.dirty_set.contains(&addr)
    }
    /// Map a page, return false if no more space
    fn map(&mut self, addr: VirtAddr) -> bool {
        if self.mapped_set.len() == self.capacity {
            return false;
        }
        self.mapped_set.insert(addr);
        true
    }
    fn unmap(&mut self, addr: VirtAddr) {
        self.mapped_set.remove(&addr);
    }
}

impl MockPageTable {
    pub fn new(capacity: usize, page_fault_handler: PageFaultHandler) -> Self {
        MockPageTable {
            mapped_set: BTreeSet::<VirtAddr>::new(),
            accessed_set: BTreeSet::<VirtAddr>::new(),
            dirty_set: BTreeSet::<VirtAddr>::new(),
            page_fault_handler,
            capacity,
        }
    }
    /// Read memory, mark accessed, trigger page fault if not present
    pub fn read(&mut self, addr: VirtAddr) {
        while !self.mapped_set.contains(&addr) {
            (self.page_fault_handler)(self, addr);
        }
        self.accessed_set.insert(addr);

    }
    /// Write memory, mark accessed and dirty, trigger page fault if not present
    pub fn write(&mut self, addr: VirtAddr) {
        while !self.mapped_set.contains(&addr) {
            (self.page_fault_handler)(self, addr);
        }
        self.accessed_set.insert(addr);
        self.dirty_set.insert(addr);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    static mut PGFAULT_COUNT: usize = 0;

    fn assert_pgfault_eq(x: usize) {
        assert_eq!(unsafe{ PGFAULT_COUNT }, x);
    }

    #[test]
    fn test() {
        fn page_fault_handler(pt: &mut MockPageTable, addr: VirtAddr) {
            unsafe{ PGFAULT_COUNT += 1; }
            pt.map(addr);
        }
        let mut pt = MockPageTable::new(2, page_fault_handler);

        pt.map(0);
        pt.read(0);
        assert_pgfault_eq(0);
        assert!(pt.accessed(0));
        assert!(!pt.dirty(0));

        pt.write(1);
        assert_pgfault_eq(1);
        assert!(pt.accessed(1));
        assert!(pt.dirty(1));

        assert_eq!(pt.map(2), false);

        pt.unmap(0);
        pt.read(0);
        assert_pgfault_eq(2);
    }
}