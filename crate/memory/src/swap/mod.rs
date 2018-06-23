use super::*;
use super::paging::*;
use core::ops::{Deref, DerefMut};

//pub use self::fifo::FifoSwapManager;
//pub use self::enhanced_clock::EnhancedClockSwapManager;

//mod fifo;
//mod enhanced_clock;
#[cfg(test)]
mod mock_swapper;

/// Manage all swappable pages, decide which to swap out
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
    /// Allocate space on device and write data to it.
    /// Return a token indicating the location.
    fn swap_out(&mut self, data: &[u8]) -> Result<usize, ()>;
    /// Update data on device.
    fn swap_update(&mut self, token: usize, data: &[u8]) -> Result<(), ()>;
    /// Recover data from device and deallocate the space.
    fn swap_in(&mut self, token: usize, data: &mut [u8]) -> Result<(), ()>;
}

/// Wrapper for page table, supporting swap functions
struct SwapExt<T: PageTable, M: SwapManager, S: Swapper> {
    page_table: T,
    swap_manager: M,
    swapper: S,
}

impl<T: PageTable, M: SwapManager, S: Swapper> SwapExt<T, M, S> {
    pub fn new(page_table: T, swap_manager: M, swapper: S) -> Self {
        SwapExt {
            page_table,
            swap_manager,
            swapper,
        }
    }
    /// Swap out any one of the swapped pages, return the released PhysAddr.
    pub fn swap_out_any(&mut self) -> Result<PhysAddr, SwapError> {
        match self.swap_manager.pop() {
            None => Err(SwapError::NoSwapped),
            Some(addr) => self.swap_out(addr),
        }
    }
    /// Swap out page of `addr`, return the origin map target.
    fn swap_out(&mut self, addr: VirtAddr) -> Result<PhysAddr, SwapError> {
        let data = self.page_table.get_page_slice_mut(addr);
        let entry = self.page_table.get_entry(addr);
        if entry.swapped() {
            return Err(SwapError::AlreadySwapped);
        }
        let token = self.swapper.swap_out(data).map_err(|_| SwapError::IOError)?;
        let target = entry.target();
        entry.set_target(token * PAGE_SIZE);
        entry.set_swapped(true);
        entry.set_present(false);
        entry.update();
        Ok(target)
    }
    /// Map page of `addr` to `target`, then swap in the data.
    fn swap_in(&mut self, addr: VirtAddr, target: PhysAddr) -> Result<(), SwapError> {
        let token = {
            let entry = self.page_table.get_entry(addr);
            if !entry.swapped() {
                return Err(SwapError::NotSwapped);
            }
            let token = entry.target() / PAGE_SIZE;
            entry.set_target(target);
            entry.set_swapped(false);
            entry.set_present(true);
            entry.update();
            token
        };
        let data = self.page_table.get_page_slice_mut(addr);
        self.swapper.swap_in(token, data).map_err(|_| SwapError::IOError)?;
        Ok(())
    }
    pub fn page_fault_handler(&mut self, addr: VirtAddr, alloc_frame: impl FnOnce() -> PhysAddr) -> bool {
        {
            let entry = self.page_table.get_entry(addr);
            if !entry.swapped() {
                return false;
            }
        }
        self.swap_in(addr, alloc_frame()).ok().unwrap();
        true
    }
}

pub enum SwapError {
    AlreadySwapped,
    NotSwapped,
    NoSwapped,
    IOError,
}

impl<T: PageTable, M: SwapManager, S: Swapper> Deref for SwapExt<T, M, S> {
    type Target = T;

    fn deref(&self) -> &<Self as Deref>::Target {
        &self.page_table
    }
}

impl<T: PageTable, M: SwapManager, S: Swapper> DerefMut for SwapExt<T, M, S> {
    fn deref_mut(&mut self) -> &mut <Self as Deref>::Target {
        &mut self.page_table
    }
}