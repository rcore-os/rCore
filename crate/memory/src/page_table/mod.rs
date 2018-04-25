use super::*;

pub mod mock_page_table;

pub trait PageTable {
    fn accessed(&self, addr: VirtAddr) -> bool;
    fn dirty(&self, addr: VirtAddr) -> bool;
    fn clear_accessed(&mut self, addr: VirtAddr);
    fn clear_dirty(&mut self, addr: VirtAddr);
    fn map(&mut self, addr: VirtAddr) -> bool;
    fn unmap(&mut self, addr: VirtAddr);
}