use super::*;
use super::page_table::PageTable;

pub mod fifo;
mod mock_swapper;

trait SwapManager {
    /// Called when tick interrupt occured
    fn tick(&mut self);
    /// Called when map a swappable page into the memory
    fn push(&mut self, addr: VirtAddr);
    /// Called to delete the addr entry from the swap manager
    fn remove(&mut self, addr: VirtAddr);
    /// Try to swap out a page, return then victim
    fn pop(&mut self) -> Option<VirtAddr>;
}

trait Swapper {
    fn swap_out(&mut self, data: &[u8; 4096]) -> usize;
    fn swap_in(&mut self, token: usize, data: &mut [u8; 4096]);
}