use alloc::boxed::Box;
use super::*;

const PAGE_COUNT: usize = 16;
const PAGE_SIZE: usize = 4096;

pub struct MockPageTable {
    entries: [MockEntry; PAGE_COUNT],
    data: [u8; PAGE_SIZE * PAGE_COUNT],
    page_fault_handler: Option<PageFaultHandler>,
}

#[derive(Default, Copy, Clone)]
pub struct MockEntry {
    target: PhysAddr,
    present: bool,
    writable: bool,
    accessed: bool,
    dirty: bool,
}

impl Entry for MockEntry {
    fn accessed(&self) -> bool { self.accessed }
    fn dirty(&self) -> bool { self.dirty }
    fn writable(&self) -> bool { self.writable }
    fn present(&self) -> bool { self.present }
    fn clear_accessed(&mut self) { self.accessed = false; }
    fn clear_dirty(&mut self) { self.dirty = false; }
    fn set_writable(&mut self, value: bool) { self.writable = value; }
    fn set_present(&mut self, value: bool) { self.present = value; }
    fn target(&self) -> usize { self.target }
}

type PageFaultHandler = Box<FnMut(&mut MockPageTable, VirtAddr)>;

impl PageTable for MockPageTable {
    type Entry = MockEntry;

    /// Map a page, return false if no more space
    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut Self::Entry {
        let entry = &mut self.entries[addr / PAGE_SIZE];
        assert!(!entry.present);
        entry.present = true;
        entry.target = target & !(PAGE_SIZE - 1);
        entry
    }
    fn unmap(&mut self, addr: VirtAddr) {
        let entry = &mut self.entries[addr / PAGE_SIZE];
        assert!(entry.present);
        entry.present = false;
    }

    fn get_entry(&mut self, addr: VirtAddr) -> &mut <Self as PageTable>::Entry {
        &mut self.entries[addr / PAGE_SIZE]
    }
}

impl MockPageTable {
    pub fn new() -> Self {
        use core::mem::uninitialized;
        MockPageTable {
            entries: [MockEntry::default(); PAGE_COUNT],
            data: unsafe { uninitialized() },
            page_fault_handler: None,
        }
    }
    pub fn set_handler(&mut self, page_fault_handler: PageFaultHandler) {
        self.page_fault_handler = Some(page_fault_handler);
    }
    fn trigger_page_fault(&mut self, addr: VirtAddr) {
        // In order to call the handler with &mut self as an argument
        // We have to first take the handler out of self, finally put it back
        let mut handler = self.page_fault_handler.take().unwrap();
        handler(self, addr);
        self.page_fault_handler = Some(handler);
    }
    fn translate(&self, addr: VirtAddr) -> PhysAddr {
        let entry = &self.entries[addr / PAGE_SIZE];
        assert!(entry.present);
        (entry.target & !(PAGE_SIZE - 1)) | (addr & (PAGE_SIZE - 1))
    }
    /// Read memory, mark accessed, trigger page fault if not present
    pub fn read(&mut self, addr: VirtAddr) -> u8 {
        while !self.entries[addr / PAGE_SIZE].present {
            self.trigger_page_fault(addr);
        }
        self.entries[addr / PAGE_SIZE].accessed = true;
        self.data[self.translate(addr)]
    }
    /// Write memory, mark accessed and dirty, trigger page fault if not present
    pub fn write(&mut self, addr: VirtAddr, data: u8) {
        while !(self.entries[addr / PAGE_SIZE].present && self.entries[addr / PAGE_SIZE].writable) {
            self.trigger_page_fault(addr);
        }
        self.entries[addr / PAGE_SIZE].accessed = true;
        self.entries[addr / PAGE_SIZE].dirty = true;
        self.data[self.translate(addr)] = data;
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

        let mut pt = MockPageTable::new();
        pt.set_handler(Box::new({
            let page_fault_count1 = page_fault_count.clone();
            move |pt: &mut MockPageTable, addr: VirtAddr| {
                *page_fault_count1.borrow_mut() += 1;
                pt.map(addr, addr).set_writable(true);
            }
        }));

        pt.map(0, 0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 0);
        assert!(pt.get_entry(0).accessed());
        assert!(!pt.get_entry(0).dirty());

        pt.get_entry(0).clear_accessed();
        assert!(!pt.get_entry(0).accessed());

        pt.read(1);
        assert_eq!(*page_fault_count.borrow(), 0);
        assert!(pt.get_entry(0).accessed());

        pt.write(0x1000, 0xff);
        assert_eq!(*page_fault_count.borrow(), 1);
        assert!(pt.get_entry(0x1000).accessed());
        assert!(pt.get_entry(0x1000).dirty());
        assert_eq!(pt.read(0x1000), 0xff);

        pt.get_entry(0x1000).clear_dirty();
        assert!(!pt.get_entry(0x1000).dirty());

        pt.unmap(0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 2);
    }
}