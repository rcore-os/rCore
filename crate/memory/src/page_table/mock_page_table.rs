use alloc::{boxed::Box, btree_set::BTreeSet};
use super::*;

pub struct MockPageTable {
    mapped_set: BTreeSet<VirtAddr>,
    accessed_set: BTreeSet<VirtAddr>,
    dirty_set: BTreeSet<VirtAddr>,
    page_fault_handler: PageFaultHandler,
    capacity: usize,
}

type PageFaultHandler = Box<FnMut(&mut MockPageTable, VirtAddr)>;

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
            let self_mut = unsafe{ &mut *(self as *mut Self) };
            (self.page_fault_handler)(self_mut, addr);
        }
        self.accessed_set.insert(addr);

    }
    /// Write memory, mark accessed and dirty, trigger page fault if not present
    pub fn write(&mut self, addr: VirtAddr) {
        while !self.mapped_set.contains(&addr) {
            let self_mut = unsafe{ &mut *(self as *mut Self) };
            (self.page_fault_handler)(self_mut, addr);
        }
        self.accessed_set.insert(addr);
        self.dirty_set.insert(addr);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::arc::Arc;
    use core::cell::RefCell;

    #[test]
    fn test() {
        let page_fault_count = Arc::new(RefCell::new(0usize));

        let mut pt = MockPageTable::new(2, Box::new({
            let page_fault_count1 = page_fault_count.clone();
            move |pt: &mut MockPageTable, addr: VirtAddr| {
                *page_fault_count1.borrow_mut() += 1;
                pt.map(addr);
            }
        }));

        pt.map(0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 0);
        assert!(pt.accessed(0));
        assert!(!pt.dirty(0));

        pt.write(1);
        assert_eq!(*page_fault_count.borrow(), 1);
        assert!(pt.accessed(1));
        assert!(pt.dirty(1));

        assert_eq!(pt.map(2), false);

        pt.unmap(0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 2);
    }
}