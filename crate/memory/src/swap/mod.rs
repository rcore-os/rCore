//! Swap extension for page table
//! and generic interface for swap manager and swapper
//!
//! To use the SwapExt, make a wrapper over the original apge table using swap manager and swapper
//! Like: SwapExt::new(origin_page_table,swap_manager,swapper)
//! Invoke page_fault_handler() on the SwapExt to run the swap process
//! If the method above returns true, a page is swapped in, else do your own things.

use super::*;
use super::paging::*;
use core::ops::{Deref, DerefMut};

//pub use self::fifo::FifoSwapManager;
pub use self::enhanced_clock::EnhancedClockSwapManager;

pub mod fifo;
mod enhanced_clock;
pub mod mock_swapper;
//#[cfg(test)]
//mod mock_swapper;

/// Manage all swappable pages, decide which to swap out
pub trait SwapManager {
    /*
    **  @brief  update intarnal state pre tick
    **          Called when tick interrupt occured
    **  @retval none
    */
    fn tick(&mut self);
    /*
    **  @brief  update intarnal state when page is pushed into memory
    **          Called when map a swappable page into the memory
    **  @param  addr: VirtAddr       the virual address of the page pushed into memory
    **  @retval none
    */
    fn push(&mut self, addr: VirtAddr);
    /*
    **  @brief  update intarnal state when page is removed from memory
    **          Called to delete the addr entry from the swap manager
    **  @param  addr: VirtAddr       the virual address of the page removed from memory
    **  @retval none
    */
    fn remove(&mut self, addr: VirtAddr);
    /*
    **  @brief  select swap out victim when there is need to swap out a page
    **          (The params is only used by `EnhancedClockSwapManager` currently)
    **  @param  page_table: &mut T   the current page table
    **  @param  swapper: &mut S      the swapper used
    **  @retval Option<VirtAddr>     the virual address of the victim page, if present
    */
    fn pop<T, S>(&mut self, page_table: &mut T, swapper: &mut S) -> Option<VirtAddr>
        where T: PageTable, S: Swapper;
}

/// Implement swap in & out execution
pub trait Swapper {
    /*
    **  @brief  Allocate space on device and write data to it
    **  @param  data: &[u8]          the data to write to the device
    **  @retval Result<usize, ()>    the execute result, and a token indicating the location on the device if success
    */
    fn swap_out(&mut self, data: &[u8]) -> Result<usize, ()>;
    /*
    **  @brief  Update data on device.
    **  @param  token: usize         the token indicating the location on the device
    **  @param  data: &[u8]          the data to overwrite on the device
    **  @retval Result<(), ()>       the execute result
    */
    fn swap_update(&mut self, token: usize, data: &[u8]) -> Result<(), ()>;
    /*
    **  @brief  Recover data from device and deallocate the space.
    **  @param  token: usize         the token indicating the location on the device
    **  @param  data: &mut [u8]      the reference to data in the space in memory
    **  @retval Result<(), ()>       the execute result
    */
    fn swap_in(&mut self, token: usize, data: &mut [u8]) -> Result<(), ()>;
}

/// Wrapper for page table, supporting swap functions
pub struct SwapExt<T: PageTable, M: SwapManager, S: Swapper> {
    page_table: T,
    swap_manager: M,
    swapper: S,
}

impl<T: PageTable, M: SwapManager, S: Swapper> SwapExt<T, M, S> {
    /*
    **  @brief  create a swap extension
    **  @param  page_table: T        the inner page table
    **  @param  swap_manager: M      the SwapManager used
    **  @param  swapper: S           the Swapper used
    **  @retval SwapExt              the swap extension created
    */
    pub fn new(page_table: T, swap_manager: M, swapper: S) -> Self {
        SwapExt {
            page_table,
            swap_manager,
            swapper,
        }
    }
    /*
    **  @brief  map the virtual address to a target physics address as swappable
    **  @param  addr: VirtAddr       the virual address to map
    **  @param  target: VirtAddr     the target physics address
    **  @retval none
    */
    pub fn map_to_swappable(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut T::Entry {
        self.swap_manager.push(addr);
        self.map(addr, target)
    }
    /*
    **  @brief  Swap out any one of the swapped pages
    **  @retval Result<PhysAddr, SwapError>
    **                               the physics address of released frame if success,
    **                               the error if failed
    */
    pub fn swap_out_any(&mut self) -> Result<PhysAddr, SwapError> {
        let victim = {
            let Self {ref mut page_table, ref mut swap_manager, ref mut swapper} = self;
            swap_manager.pop(page_table, swapper)
        };
        match victim {
            None => Err(SwapError::NoSwapped),
            Some(addr) => self.swap_out(addr),
        }
    }
    /*
    **  @brief  Swap out page
    **  @param  addr: VirtAddr       the virual address of beginning of page
    **  @retval Result<PhysAddr, SwapError>
    **                               the physics address of the original map target frame if success,
    **                               the error if failed
    */
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
    /*
    **  @brief  map the virtual address to a target physics address and then swap in page data
    **  @param  addr: VirtAddr       the virual address of beginning of page
    **  @param  addr: PhysAddr       the target physics address
    **  @retval Result<()), SwapError>
    **                               the execute result, and the error if failed
    */
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
        self.swap_manager.push(addr);
        Ok(())
    }
    /*
    **  @brief  execute the swap process for page fault
    **          This function must be called whenever PageFault happens.
    **  @param  addr: VirtAddr       the virual address of the page fault
    **  @param  alloc_frame: impl FnOnce() -> PhysAddr
    **                               the page allocation function
    **                               that allocate a page and returns physics address
    **                               of beginning of the page
    **  @retval bool                 whether swap in happens.
    */
    pub fn page_fault_handler(&mut self, addr: VirtAddr, alloc_frame: impl FnOnce() -> Option<PhysAddr>) -> bool {
        if !self.page_table.get_entry(addr).swapped() {
            return false;
        }
        // Allocate a frame, if failed, swap out a page
        let frame = alloc_frame().unwrap_or_else(|| self.swap_out_any().ok().unwrap());
        self.swap_in(addr, frame).ok().unwrap();
        true
    }
}

pub enum SwapError {
    /// attempt to swap out a page that is already swapped out
    AlreadySwapped,
    /// attempt to swap in a page that is already in the memory
    NotSwapped,
    /// there are no page to be swapped out
    NoSwapped,
    /// swap failed due to IO error while interact with device
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

#[cfg(test)]
mod test {
    use super::*;
    use super::mock_swapper::MockSwapper;
    use alloc::{arc::Arc, boxed::Box};
    use core::cell::RefCell;
    use paging::MockPageTable;

    #[derive(Debug)]
    pub enum MemOp {
        R(usize),
        W(usize),
    }

    struct FrameAlloc(usize);

    impl FrameAlloc {
        fn alloc(&mut self) -> Option<PhysAddr> {
            if self.0 == 0 {
                return None;
            }
            self.0 -= 1;
            Some((self.0 + 1) * PAGE_SIZE)
        }
    }

    unsafe fn clone<'a, 'b, T>(x: &'a mut T) -> &'b mut T {
        &mut *(x as *mut T)
    }

    /// Test framework with different SwapManagers.
    /// See `fifo::test` mod for example.
    pub fn test_manager(swap_manager: impl 'static + SwapManager, ops: &[MemOp], pgfault_count: &[u8]) {
        use self::MemOp::{R, W};
        let page_fault_count = Arc::new(RefCell::new(0u8));

        let mut pt = SwapExt::new(MockPageTable::new(), swap_manager, MockSwapper::default());

        // Move to closure
        let pt0 = unsafe{ clone(&mut pt) };
        let page_fault_count1 = page_fault_count.clone();
        let mut alloc = FrameAlloc(4);

        pt.set_handler(Box::new(move |_, addr: VirtAddr| {
            *page_fault_count1.borrow_mut() += 1;
            if pt0.page_fault_handler(addr, || alloc.alloc()) {
                return;
            }
            // The page is not mapped, map it to a new frame, if no more frame, swap out.
            let target = alloc.alloc().or_else(|| pt0.swap_out_any().ok())
                .expect("no more frame in both allocator and swap_manager");
            pt0.map_to_swappable(addr, target);
        }));

        for (op, &count) in ops.iter().zip(pgfault_count.iter()) {
            match op {
                R(addr) => { pt.read(*addr); }
                W(addr) => pt.write(*addr, 0),
            }
            assert_eq!(*(*page_fault_count).borrow(), count);
        }
    }
}