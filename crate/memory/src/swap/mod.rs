use super::*;
use super::paging::*;
use core::ops::{Deref, DerefMut};

//pub use self::fifo::FifoSwapManager;
pub use self::enhanced_clock::EnhancedClockSwapManager;

mod fifo;
mod enhanced_clock;
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
    /// (The params is only used by `EnhancedClockSwapManager`)
    fn pop<T, S>(&mut self, page_table: &mut T, swapper: &mut S) -> Option<VirtAddr>
        where T: PageTable, S: Swapper;
}

/// Do swap in & out
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
    pub fn map_to_swappable(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut T::Entry {
        self.swap_manager.push(addr);
        self.map(addr, target)
    }
    /// Swap out any one of the swapped pages, return the released PhysAddr.
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
        self.swap_manager.push(addr);
        Ok(())
    }
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

#[cfg(test)]
mod test {
    use super::*;
    use super::mock_swapper::MockSwapper;
    use alloc::{sync::Arc, boxed::Box};
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