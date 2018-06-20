use super::*;
pub use self::mock_page_table::MockPageTable;

mod mock_page_table;

pub trait PageTable {
    type Entry: Entry;
    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut Self::Entry;
    fn unmap(&mut self, addr: VirtAddr);
    fn get_entry(&mut self, addr: VirtAddr) -> &mut Self::Entry;
}

pub trait Entry {
    fn accessed(&self) -> bool;
    // Will be set when accessed
    fn dirty(&self) -> bool;
    // Will be set when written
    fn writable(&self) -> bool;
    // Will PageFault when try to write page where writable=0
    fn present(&self) -> bool;      // Will PageFault when try to access page where present=0

    fn clear_accessed(&mut self);
    fn clear_dirty(&mut self);
    fn set_writable(&mut self, value: bool);
    fn set_present(&mut self, value: bool);

    fn target(&self) -> PhysAddr;
}
