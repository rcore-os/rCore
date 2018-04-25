pub use self::fifo::FifoSwapManager;
pub use self::enhanced_clock::EnhancedClockSwapManager;

use super::*;
use super::page_table::PageTable;

mod fifo;
mod enhanced_clock;
mod mock_swapper;

pub trait SwapManager {
    /// Called when tick interrupt occured
    fn tick(&mut self);
    /// Called when map a swappable page into the memory
    fn push(&mut self, addr: VirtAddr);
    /// Called to delete the addr entry from the swap manager
    fn remove(&mut self, addr: VirtAddr);
    /// Try to swap out a page, return then victim
    fn pop(&mut self) -> Option<VirtAddr>;
}

pub trait Swapper {
    fn swap_out(&mut self, data: &[u8; 4096]) -> Result<usize, ()>;
    fn swap_update(&mut self, token: usize, data: &[u8; 4096]) -> Result<(), ()>;
    fn swap_in(&mut self, token: usize, data: &mut [u8; 4096]) -> Result<(), ()>;
}

pub trait SwappablePageTable: PageTable {
    fn swap_out(&mut self, addr: VirtAddr) -> Result<(), ()>;
}