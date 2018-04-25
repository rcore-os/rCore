use alloc::boxed::Box;
use super::*;

const PAGE_COUNT: usize = 16;
const PAGE_SIZE: usize = 4096;

pub struct MockPageTable {
    mapped: [bool; PAGE_COUNT],
    accessed: [bool; PAGE_COUNT],
    dirty: [bool; PAGE_COUNT],
    data: [u8; PAGE_SIZE * PAGE_COUNT],
    page_fault_handler: Option<PageFaultHandler>,
    capacity: usize,
}

type PageFaultHandler = Box<FnMut(&mut MockPageTable, VirtAddr)>;

impl PageTable for MockPageTable {
    fn accessed(&self, addr: VirtAddr) -> bool {
        self.accessed[addr / PAGE_SIZE]
    }
    fn dirty(&self, addr: VirtAddr) -> bool {
        self.dirty[addr / PAGE_SIZE]
    }
    fn clear_accessed(&mut self, addr: usize) {
        self.accessed[addr / PAGE_SIZE] = false;
    }
    fn clear_dirty(&mut self, addr: usize) {
        self.dirty[addr / PAGE_SIZE] = false;
    }
    /// Map a page, return false if no more space
    fn map(&mut self, addr: VirtAddr) -> bool {
        if self.mapped.iter().filter(|&&b| b).count() == self.capacity {
            return false;
        }
        self.mapped[addr / PAGE_SIZE] = true;
        true
    }
    fn unmap(&mut self, addr: VirtAddr) {
        self.mapped[addr / PAGE_SIZE] = false;
    }
}

impl MockPageTable {
    pub fn new(capacity: usize) -> Self {
        use core::mem::uninitialized;
        MockPageTable {
            mapped: [false; PAGE_COUNT],
            accessed: [false; PAGE_COUNT],
            dirty: [false; PAGE_COUNT],
            data: unsafe{ uninitialized() },
            page_fault_handler: None,
            capacity,
        }
    }
    pub fn set_handler(&mut self, page_fault_handler: PageFaultHandler) {
        self.page_fault_handler = Some(page_fault_handler);
    }
    fn trigger_page_fault_if_not_present(&mut self, addr: VirtAddr) {
        let page_id = addr / PAGE_SIZE;
        while !self.mapped[page_id] {
            let self_mut = unsafe{ &mut *(self as *mut Self) };
            (self.page_fault_handler.as_mut().unwrap())(self_mut, addr);
        }
    }
    /// Read memory, mark accessed, trigger page fault if not present
    pub fn read(&mut self, addr: VirtAddr) -> u8 {
        let page_id = addr / PAGE_SIZE;
        self.trigger_page_fault_if_not_present(addr);
        self.accessed[page_id] = true;
        self.data[addr]
    }
    /// Write memory, mark accessed and dirty, trigger page fault if not present
    pub fn write(&mut self, addr: VirtAddr, data: u8) {
        let page_id = addr / PAGE_SIZE;
        self.trigger_page_fault_if_not_present(addr);
        self.accessed[page_id] = true;
        self.dirty[page_id] = true;
        self.data[addr] = data;
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

        let mut pt = MockPageTable::new(2);
        pt.set_handler(Box::new({
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

        pt.clear_accessed(0);
        assert!(!pt.accessed(0));

        pt.read(1);
        assert_eq!(*page_fault_count.borrow(), 0);
        assert!(pt.accessed(0));

        pt.write(0x1000, 0xff);
        assert_eq!(*page_fault_count.borrow(), 1);
        assert!(pt.accessed(0x1000));
        assert!(pt.dirty(0x1000));
        assert_eq!(pt.read(0x1000), 0xff);

        pt.clear_dirty(0x1000);
        assert!(!pt.dirty(0x1000));

        assert_eq!(pt.map(0x2000), false);

        pt.unmap(0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 2);
    }
}