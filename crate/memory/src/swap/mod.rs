//! Swap extension for page table
//! and generic interface for swap manager and swapper
//!
//! To use the SwapExt, make a wrapper over the original apge table using swap manager and swapper
//! Like: SwapExt::new(origin_page_table,swap_manager,swapper)
//! Invoke page_fault_handler() on the SwapExt to run the swap process
//! If the method above returns true, a page is swapped in, else do your own things.

use super::addr::Frame;
use super::paging::*;
use super::*;
use core::ops::{Deref, DerefMut};

//pub use self::fifo::FifoSwapManager;
//pub use self::enhanced_clock::EnhancedClockSwapManager;

pub mod fifo;
//mod enhanced_clock;
pub mod mock_swapper;
//#[cfg(test)]
//mod mock_swapper;

/// Manage all swappable pages, decide which to swap out
pub trait SwapManager {
    //type Inactive: InactivePageTable;
    /*
     **  @brief  update intarnal state pre tick
     **          Called when tick interrupt occured
     **  @retval none
     */
    fn tick(&mut self);
    /*
     **  @brief  update intarnal state when page is pushed into memory
     **          Called when map a swappable page into the memory
     **  @param  frame: Frame       the Frame recording the swappable frame info
     **  @retval none
     */
    fn push(&mut self, frame: Frame);
    /*
     **  @brief  update intarnal state when page is removed from memory
     **          Called to delete the addr entry from the swap manager
     **  @param  token: usize         the inactive page table token for the virtual address
     **  @param  addr: VirtAddr       the virual address of the page removed from memory
     **  @retval none
     */
    fn remove(&mut self, token: usize, addr: VirtAddr);
    /*
     **  @brief  select swap out victim when there is need to swap out a page
     **          (The params is only used by `EnhancedClockSwapManager` currently)
     **  @param  page_table: &mut T   the current page table
     **  @param  swapper: &mut S      the swapper used
     **  @retval Option<Frame>     the Frame of the victim page, if present
     */
    fn pop<T, S>(&mut self, page_table: &mut T, swapper: &mut S) -> Option<Frame>
    where
        T: PageTable,
        S: Swapper;
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
     **  @brief set a page swappable
     **  @param pt: *mut T2           the raw pointer for the target page's inactive page table
     **  @param addr: VirtAddr        the target page's virtual address
     */
    pub unsafe fn set_swappable<T2: InactivePageTable>(&mut self, pt: *mut T2, addr: VirtAddr) {
        let Self {
            ref mut page_table,
            ref mut swap_manager,
            ..
        } = self;
        let targetpt = &mut *(pt);
        let pttoken = {
            info!(
                "SET_SWAPPABLE: the target page table token is {:x?}, addr is {:x?}",
                targetpt.token(),
                addr
            );
            targetpt.token()
        };
        targetpt.with(|| {
            let entry = page_table
                .get_entry(addr)
                .expect("failed to get page entry when set swappable");
            if entry.present() {
                let frame = Frame::new(pt as usize, addr, pttoken);
                swap_manager.push(frame);
            }
        });
        /*
        let token = unsafe{
            (*pt).token()
        };
        let frame = Frame::new(pt as usize, addr, token);
        self.swap_manager.push(frame);
        */
    }

    /*
     **  @brief remove a page (given virtual address) from swappable pages, if the page is swapped, swap in at first
     **  @param pt: *mut T2           the raw pointer for the target page's inactive page table
     **  @param addr: VirtAddr        the target page's virtual address
     **  @param alloc_frame:          the function to alloc a free physical frame for once
     */
    pub unsafe fn remove_from_swappable<T2: InactivePageTable>(
        &mut self,
        pt: *mut T2,
        addr: VirtAddr,
        alloc_frame: impl FnOnce() -> PhysAddr,
    ) {
        //info!("come into remove_from swappable");
        let Self {
            ref mut page_table,
            ref mut swap_manager,
            ref mut swapper,
        } = self;
        let targetpt = &mut *(pt);
        let pttoken = {
            info!(
                "SET_UNSWAPPABLE: the target page table token is {:x?}, addr is {:x?}",
                targetpt.token(),
                addr
            );
            targetpt.token()
        };
        //info!("try to change pagetable");
        targetpt.with(|| {
            let token = {
                let entry = page_table.get_entry(addr).unwrap();
                if !entry.swapped() {
                    if entry.present() {
                        // if the addr isn't indicating a swapped page, panic occured here
                        swap_manager.remove(pttoken, addr);
                    }
                    return;
                }
                let token = entry.target() / PAGE_SIZE;
                let frame = alloc_frame();
                entry.set_target(frame);
                entry.set_swapped(false);
                entry.set_present(true);
                entry.update();
                token
            };
            info!("swap in vaddr {:x?} at remove from swappable.", addr);
            let data = page_table.get_page_slice_mut(addr);
            swapper.swap_in(token, data).unwrap();
        });
        trace!("come out of femove_from swappable");
    }

    /*
     **  @brief  map the virtual address to a target physics address as swappable
     **  @param  addr: VirtAddr       the virual address to map
     **  @param  target: VirtAddr     the target physics address
     **  @retval none
     */
    /*
    pub fn map_to_swappable(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut T::Entry {
        self.swap_manager.push(addr);
        self.map(addr, target)
    }*/

    /*
     **  @brief  Swap out any one of the swapped pages
     **  @retval Result<PhysAddr, SwapError>
     **                               the physics address of released frame if success,
     **                               the error if failed
     */
    pub fn swap_out_any<T2: InactivePageTable>(&mut self) -> Result<PhysAddr, SwapError> {
        info!("COME in to swap_out_any");
        let victim: Option<Frame> = {
            let Self {
                ref mut page_table,
                ref mut swap_manager,
                ref mut swapper,
            } = self;
            swap_manager.pop(page_table, swapper)
        };
        info!("swap out page {}", victim.unwrap().get_virtaddr());
        match victim {
            None => Err(SwapError::NoSwapped),
            Some(frame) => self.swap_out::<T2>(&frame),
        }
    }

    /*
     **  @brief  Swap out page
     **  @param  frame: Frame       the Frame of page recording the page info
     **  @retval Result<PhysAddr, SwapError>
     **                               the physics address of the original map target frame if success,
     **                               the error if failed
     */
    fn swap_out<T2: InactivePageTable>(&mut self, frame: &Frame) -> Result<PhysAddr, SwapError> {
        let Self {
            ref mut page_table,
            ref mut swapper,
            ..
        } = self;
        let ret = unsafe {
            let pt = &mut *(frame.get_page_table() as *mut T2);
            pt.with(|| {
                //use core::slice;
                //let data = unsafe { slice::from_raw_parts_mut((frame.virtaddr & !(PAGE_SIZE - 1)) as *mut u8, PAGE_SIZE) };
                let data = page_table.get_page_slice_mut(frame.get_virtaddr());
                let entry = page_table
                    .get_entry(frame.get_virtaddr())
                    .ok_or(SwapError::NotMapped)?;
                if entry.swapped() {
                    return Err(SwapError::AlreadySwapped);
                }
                //assert!(!entry.swapped(), "Page already swapped!");
                let token = swapper.swap_out(data).map_err(|_| SwapError::IOError)?;
                //let token = swapper.swap_out(data).unwrap();
                let target = entry.target();
                entry.set_target(token * PAGE_SIZE);
                entry.set_swapped(true);
                entry.set_present(false);
                entry.update();
                Ok(target)
            })
        };
        ret
    }
    /*
     **  @brief  map the virtual address to a target physics address and then swap in page data, noted that the page should be in the current page table
     **  @param pt: *mut T2           the raw pointer for the swapping page's inactive page table
     **  @param  addr: VirtAddr       the virual address of beginning of page
     **  @param  target: PhysAddr       the target physics address
     **  @retval Result<()), SwapError>
     **                               the execute result, and the error if failed
     */
    fn swap_in<T2: InactivePageTable>(
        &mut self,
        pt: *mut T2,
        addr: VirtAddr,
        target: PhysAddr,
    ) -> Result<(), SwapError> {
        info!("come in to swap in");
        let entry = self
            .page_table
            .get_entry(addr)
            .ok_or(SwapError::NotMapped)?;
        if !entry.swapped() {
            return Err(SwapError::NotSwapped);
        }
        let token = entry.target() / PAGE_SIZE;
        entry.set_target(target);
        entry.set_swapped(false);
        entry.set_present(true);
        entry.update();
        let data = self.page_table.get_page_slice_mut(addr);
        self.swapper
            .swap_in(token, data)
            .map_err(|_| SwapError::IOError)?;
        let pttoken = unsafe { (*pt).token() };
        let frame = Frame::new(pt as usize, addr, pttoken);
;
        self.swap_manager.push(frame);
        Ok(())
    }
    /*
     **  @brief  execute the frame delayed allocate and  swap process for page fault
     **          This function must be called whenever PageFault happens.
     **  @param  pt: *mut T2          the raw pointer for the target page's inactive page table (exactly the current page table)
     **  @param  addr: VirtAddr       the virual address of the page fault
     **  @param  swapin: bool         whether to set the page swappable if delayed allocate a frame for a page
     **  @param  alloc_frame: impl FnOnce() -> PhysAddr
     **                               the page allocation function
     **                               that allocate a page and returns physics address
     **                               of beginning of the page
     **  @retval bool                 whether swap in happens.
     */
    pub fn page_fault_handler<T2: InactivePageTable>(
        &mut self,
        pt: *mut T2,
        addr: VirtAddr,
        swapin: bool,
        alloc_frame: impl FnOnce() -> PhysAddr,
    ) -> bool {
        // handle page delayed allocating
        {
            info!("try handling delayed frame allocator");
            let need_alloc = {
                let entry = self.page_table.get_entry(addr).expect("fail to get entry");
                //info!("got entry!");
                !entry.present() && !entry.swapped()
            };
            info!("need_alloc got");
            if need_alloc {
                info!("need_alloc!");
                let frame = alloc_frame();
                {
                    let entry = self.page_table.get_entry(addr).unwrap();
                    entry.set_target(frame);
                    //let new_entry = self.page_table.map(addr, frame);
                    entry.set_present(true);
                    entry.update();
                }
                if swapin {
                    unsafe {
                        self.set_swappable(pt, addr & 0xfffff000);
                    }
                }
                //area.get_flags().apply(new_entry); this instruction may be used when hide attr is used
                info!("allocated successfully");
                return true;
            }
            info!("not need alloc!");
        }
        // now we didn't attach the cow so the present will be false when swapped(), to enable the cow some changes will be needed
        match self.page_table.get_entry(addr) {
            // infact the get_entry(addr) should not be None here
            None => return false,
            Some(entry) => {
                if !entry.swapped() {
                    return false;
                }
            }
        }
        // Allocate a frame, if failed, swap out a page
        let frame = alloc_frame();
        self.swap_in(pt, addr, frame).ok().unwrap();
        true
    }
}

pub enum SwapError {
    /// attempt to swap out a page that is already swapped out
    AlreadySwapped,
    ///
    NotMapped,
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

/*
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
*/
