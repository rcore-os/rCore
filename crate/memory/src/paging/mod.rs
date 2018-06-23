//! Generic page table interface
//!
//! Implemented for every architecture, used by OS.

use super::*;
#[cfg(test)]
pub use self::mock_page_table::MockPageTable;

#[cfg(test)]
mod mock_page_table;

pub trait PageTable {
    type Entry: Entry;
    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut Self::Entry;
    fn unmap(&mut self, addr: VirtAddr);
    fn get_entry(&mut self, addr: VirtAddr) -> &mut Self::Entry;
    // For testing with mock
    fn read_page(&mut self, addr: VirtAddr, data: &mut [u8]);
    fn write_page(&mut self, addr: VirtAddr, data: &[u8]);
    fn read(&mut self, addr: VirtAddr) -> u8;
    fn write(&mut self, addr: VirtAddr, data: u8);
}

pub trait Entry {
    /// IMPORTANT!
    /// This must be called after any change to ensure it become effective.
    /// Usually this will make a flush to TLB/MMU.
    fn update(&mut self);

    /// Will be set when accessed
    fn accessed(&self) -> bool;
    /// Will be set when written
    fn dirty(&self) -> bool;
    /// Will PageFault when try to write page where writable=0
    fn writable(&self) -> bool;
    /// Will PageFault when try to access page where present=0
    fn present(&self) -> bool;

    fn clear_accessed(&mut self);
    fn clear_dirty(&mut self);
    fn set_writable(&mut self, value: bool);
    fn set_present(&mut self, value: bool);

    fn target(&self) -> PhysAddr;

    // For Copy-on-write extension
    fn writable_shared(&self) -> bool;
    fn readonly_shared(&self) -> bool;
    fn set_shared(&mut self, writable: bool);
    fn clear_shared(&mut self);

    fn user(&self) -> bool;
    fn set_user(&mut self, value: bool);
    fn execute(&self) -> bool;
    fn set_execute(&mut self, value: bool);
}
