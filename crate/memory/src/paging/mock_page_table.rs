//! Mock Page Table
//!
//! An mock implementation for the PageTable.
//! Used to test page table operation.

use super::*;
use alloc::boxed::Box;

const PAGE_COUNT: usize = 16;
const PAGE_SIZE: usize = 4096;

// a mock page table for test purpose
pub struct MockPageTable {
    entries: [MockEntry; PAGE_COUNT],
    data: [u8; PAGE_SIZE * PAGE_COUNT],
    page_fault_handler: Option<PageFaultHandler>,
}

// the entry of the mock page table
#[derive(Default, Copy, Clone)]
pub struct MockEntry {
    target: PhysAddr,
    present: bool,
    writable: bool,
    accessed: bool,
    dirty: bool,
    writable_shared: bool,
    readonly_shared: bool,
    swapped: bool,
}

impl Entry for MockEntry {
    fn update(&mut self) {}
    fn accessed(&self) -> bool {
        self.accessed
    }
    fn dirty(&self) -> bool {
        self.dirty
    }
    fn writable(&self) -> bool {
        self.writable
    }
    fn present(&self) -> bool {
        self.present
    }
    fn clear_accessed(&mut self) {
        self.accessed = false;
    }
    fn clear_dirty(&mut self) {
        self.dirty = false;
    }
    fn set_writable(&mut self, value: bool) {
        self.writable = value;
    }
    fn set_present(&mut self, value: bool) {
        self.present = value;
    }
    fn target(&self) -> usize {
        self.target
    }
    fn set_target(&mut self, target: usize) {
        self.target = target;
    }
    fn writable_shared(&self) -> bool {
        self.writable_shared
    }
    fn readonly_shared(&self) -> bool {
        self.readonly_shared
    }
    fn set_shared(&mut self, writable: bool) {
        self.writable_shared = writable;
        self.readonly_shared = !writable;
    }
    fn clear_shared(&mut self) {
        self.writable_shared = false;
        self.readonly_shared = false;
    }
    fn swapped(&self) -> bool {
        self.swapped
    }
    fn set_swapped(&mut self, value: bool) {
        self.swapped = value;
    }
    fn user(&self) -> bool {
        unimplemented!()
    }
    fn set_user(&mut self, _value: bool) {
        unimplemented!()
    }
    fn execute(&self) -> bool {
        unimplemented!()
    }
    fn set_execute(&mut self, _value: bool) {
        unimplemented!()
    }
    fn mmio(&self) -> u8 {
        unimplemented!()
    }
    fn set_mmio(&mut self, _value: u8) {
        unimplemented!()
    }
}

type PageFaultHandler = Box<dyn FnMut(&mut MockPageTable, VirtAddr)>;

impl PageTable for MockPageTable {
    //    type Entry = MockEntry;

    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut dyn Entry {
        let entry = &mut self.entries[addr / PAGE_SIZE];
        assert!(!entry.present);
        entry.present = true;
        entry.writable = true;
        entry.target = target & !(PAGE_SIZE - 1);
        entry
    }
    fn unmap(&mut self, addr: VirtAddr) {
        let entry = &mut self.entries[addr / PAGE_SIZE];
        assert!(entry.present);
        entry.present = false;
    }
    fn get_entry(&mut self, addr: VirtAddr) -> Option<&mut dyn Entry> {
        Some(&mut self.entries[addr / PAGE_SIZE])
    }
    fn get_page_slice_mut<'a, 'b>(&'a mut self, addr: VirtAddr) -> &'b mut [u8] {
        self._read(addr);
        let pa = self.translate(addr) & !(PAGE_SIZE - 1);
        let data = unsafe { &mut *(&mut self.data as *mut [u8; PAGE_SIZE * PAGE_COUNT]) };
        &mut data[pa..pa + PAGE_SIZE]
    }
    fn read(&mut self, addr: usize) -> u8 {
        self._read(addr);
        self.data[self.translate(addr)]
    }
    fn write(&mut self, addr: usize, data: u8) {
        self._write(addr);
        self.data[self.translate(addr)] = data;
    }
}

impl MockPageTable {
    /*
     **  @brief  create a new MockPageTable
     **  @retval MockPageTable        the mock page table created
     */
    pub fn new() -> Self {
        use core::mem::MaybeUninit;
        MockPageTable {
            entries: [MockEntry::default(); PAGE_COUNT],
            data: unsafe { MaybeUninit::zeroed().assume_init() },
            page_fault_handler: None,
        }
    }
    /*
     **  @brief  set the page fault handler
     **          used for mock the page fault feature
     **  @param  page_fault_handler: PageFaultHandler
     **                               the page fault handler
     **  @retval none
     */
    pub fn set_handler(&mut self, page_fault_handler: PageFaultHandler) {
        self.page_fault_handler = Some(page_fault_handler);
    }
    /*
     **  @brief  trigger page fault
     **          used for mock the page fault feature
     **  @param  addr: VirtAddr       the virtual address used to trigger the page fault
     **  @retval none
     */
    fn trigger_page_fault(&mut self, addr: VirtAddr) {
        // In order to call the handler with &mut self as an argument
        // We have to first take the handler out of self, finally put it back
        let mut handler = self.page_fault_handler.take().unwrap();
        handler(self, addr);
        self.page_fault_handler = Some(handler);
    }
    /*
     **  @brief  translate virtual address to physics address
     **          used for mock address translation feature
     **  @param  addr: VirtAddr       the virtual address to translation
     **  @retval PhysAddr             the translation result
     */
    fn translate(&self, addr: VirtAddr) -> PhysAddr {
        let entry = &self.entries[addr / PAGE_SIZE];
        assert!(entry.present);
        let pa = (entry.target & !(PAGE_SIZE - 1)) | (addr & (PAGE_SIZE - 1));
        assert!(pa < self.data.len(), "Physical memory access out of range");
        pa
    }
    /*
     **  @brief  attempt to read the virtual address
     **          trigger page fault when failed
     **  @param  addr: VirtAddr       the virual address of data to read
     **  @retval none
     */
    fn _read(&mut self, addr: VirtAddr) {
        while !self.entries[addr / PAGE_SIZE].present {
            self.trigger_page_fault(addr);
        }
        self.entries[addr / PAGE_SIZE].accessed = true;
    }
    /*
     **  @brief  attempt to write the virtual address
     **          trigger page fault when failed
     **  @param  addr: VirtAddr       the virual address of data to write
     **  @retval none
     */
    fn _write(&mut self, addr: VirtAddr) {
        while !(self.entries[addr / PAGE_SIZE].present && self.entries[addr / PAGE_SIZE].writable) {
            self.trigger_page_fault(addr);
        }
        self.entries[addr / PAGE_SIZE].accessed = true;
        self.entries[addr / PAGE_SIZE].dirty = true;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::sync::Arc;
    use core::cell::RefCell;

    #[test]
    fn read_write() {
        let mut pt = MockPageTable::new();
        pt.map(0x0, 0x0);
        pt.map(0x1000, 0x1000);
        pt.map(0x2000, 0x1000);

        pt.write(0x0, 1);
        pt.write(0x1, 2);
        pt.write(0x1000, 3);
        assert_eq!(pt.read(0x0), 1);
        assert_eq!(pt.read(0x1), 2);
        assert_eq!(pt.read(0x1000), 3);
        assert_eq!(pt.read(0x2000), 3);
    }

    #[test]
    fn entry() {
        let mut pt = MockPageTable::new();
        pt.map(0x0, 0x1000);

        let entry = pt.get_entry(0).unwrap();
        assert!(entry.present());
        assert!(entry.writable());
        assert!(!entry.accessed());
        assert!(!entry.dirty());
        assert_eq!(entry.target(), 0x1000);

        pt.read(0x0);
        let entry = pt.get_entry(0).unwrap();
        assert!(entry.accessed());
        assert!(!entry.dirty());

        entry.clear_accessed();
        assert!(!entry.accessed());

        pt.write(0x1, 1);
        let entry = pt.get_entry(0).unwrap();
        assert!(entry.accessed());
        assert!(entry.dirty());

        entry.clear_dirty();
        assert!(!entry.dirty());

        entry.set_writable(false);
        assert!(!entry.writable());

        entry.set_present(false);
        assert!(!entry.present());
    }

    #[test]
    fn page_fault() {
        let page_fault_count = Arc::new(RefCell::new(0usize));

        let mut pt = MockPageTable::new();
        pt.set_handler(Box::new({
            let page_fault_count1 = page_fault_count.clone();
            move |pt: &mut MockPageTable, addr: VirtAddr| {
                *page_fault_count1.borrow_mut() += 1;
                pt.map(addr, addr);
            }
        }));

        pt.map(0, 0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 0);

        pt.write(0x1000, 0xff);
        assert_eq!(*page_fault_count.borrow(), 1);

        pt.unmap(0);
        pt.read(0);
        assert_eq!(*page_fault_count.borrow(), 2);
    }
}
