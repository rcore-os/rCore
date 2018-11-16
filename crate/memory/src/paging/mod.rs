//! Generic page table interface
//!
//! Implemented for every architecture, used by OS.

use super::*;
use super::memory_set::InactivePageTable;
#[cfg(test)]
pub use self::mock_page_table::MockPageTable;

#[cfg(test)]
mod mock_page_table;

// trait for PageTable
pub trait PageTable {
    type Entry: Entry;
    /*
    **  @brief  map a virual address to the target physics address
    **  @param  addr: VirtAddr       the virual address to map
    **  @param  target: VirtAddr     the target physics address
    **  @retval Entry                the page table entry of the mapped virual address
    */
    fn map(&mut self, addr: VirtAddr, target: PhysAddr) -> &mut Self::Entry;
    /*
    **  @brief  unmap a virual address from physics address
    **  @param  addr: VirtAddr       the virual address to unmap
    **  @retval none
    */
    fn unmap(&mut self, addr: VirtAddr);
    /*
    **  @brief  get the page table entry of a virual address
    **  @param  addr: VirtAddr       the virual address
    **  @retval Entry                the page table entry of the virual address
    */
    fn get_entry(&mut self, addr: VirtAddr) -> Option<&mut Self::Entry>;
    // For testing with mock
    /*
    **  @brief  used for testing with mock
    **          get a mutable reference of the content of a page from a virtual address
    **  @param  addr: VirtAddr       the virual address of the page
    **  @retval &'b mut [u8]         mutable reference of the content of a page as array of bytes
    */
    fn get_page_slice_mut<'a,'b>(&'a mut self, addr: VirtAddr) -> &'b mut [u8];
    /*
    **  @brief  used for testing with mock
    **          read data from a virtual address
    **  @param  addr: VirtAddr       the virual address of data to read
    **  @retval u8                   the data read
    */
    fn read(&mut self, addr: VirtAddr) -> u8;
    /*
    **  @brief  used for testing with mock
    **          write data to a virtual address
    **  @param  addr: VirtAddr       the virual address of data to write
    **  @param  data: u8             the data to write
    **  @retval none
    */
    fn write(&mut self, addr: VirtAddr, data: u8);
}


// trait for Entry in PageTable
pub trait Entry {
    /*
    **  @brief  force update this page table entry
    **          IMPORTANT!
    **          This must be called after any change to ensure it become effective.
    **          Usually this will make a flush to TLB/MMU.
    **  @retval none
    */
    fn update(&mut self);
    /*
    **  @brief  get the accessed bit of the entry
    **          Will be set when accessed
    **  @retval bool                 the accessed bit
    */
    fn accessed(&self) -> bool;
    /*
    **  @brief  get the dirty bit of the entry
    **          Will be set when written
    **  @retval bool                 the dirty bit
    */
    fn dirty(&self) -> bool;
    /*
    **  @brief  get the writable bit of the entry
    **          Will PageFault when try to write page where writable=0
    **  @retval bool                 the writable bit
    */
    fn writable(&self) -> bool;
    /*
    **  @brief  get the present bit of the entry
    **          Will PageFault when try to access page where present=0
    **  @retval bool                 the present bit
    */
    fn present(&self) -> bool;


    /*
    **  @brief  clear the accessed bit
    **  @retval none
    */
    fn clear_accessed(&mut self);
    /*
    **  @brief  clear the dirty bit
    **  @retval none
    */
    fn clear_dirty(&mut self);
    /*
    **  @brief  set value of writable bit
    **  @param  value: bool          the writable bit value
    **  @retval none
    */
    fn set_writable(&mut self, value: bool);
    /*
    **  @brief  set value of present bit
    **  @param  value: bool          the present bit value
    **  @retval none
    */
    fn set_present(&mut self, value: bool);

    /*
    **  @brief  get the target physics address in the entry
    **          can be used for other purpose if present=0
    **  @retval target: PhysAddr     the target physics address
    */
    fn target(&self) -> PhysAddr;
    /*
    **  @brief  set the target physics address in the entry
    **  @param  target: PhysAddr     the target physics address
    **  @retval none
    */
    fn set_target(&mut self, target: PhysAddr);

    // For Copy-on-write extension
    /*
    **  @brief  used for Copy-on-write extension
    **          get the writable and shared bit
    **  @retval value: bool          the writable and shared bit
    */
    fn writable_shared(&self) -> bool;
    /*
    **  @brief  used for Copy-on-write extension
    **          get the readonly and shared bit
    **  @retval value: bool          the readonly and shared bit
    */
    fn readonly_shared(&self) -> bool;
    /*
    **  @brief  used for Copy-on-write extension
    **          mark the page as (writable or readonly) shared
    **  @param  writable: bool       if it is true, set the page as writable and shared
    **                               else set the page as readonly and shared
    **  @retval value: none
    */
    fn set_shared(&mut self, writable: bool);
    /*
    **  @brief  used for Copy-on-write extension
    **          mark the page as not shared
    **  @retval value: none
    */
    fn clear_shared(&mut self);

    // For Swap extension
    /*
    **  @brief  used for Swap extension
    **          get the swapped bit
    **  @retval value: bool          the swapped bit
    */
    fn swapped(&self) -> bool;
    /*
    **  @brief  used for Swap extension
    **          set the swapped bit
    **  @param  value: bool          the swapped bit value
    **  @retval none
    */
    fn set_swapped(&mut self, value: bool);

    /*
    **  @brief  get the user bit of the entry
    **  @retval bool                 the user bit
    */
    fn user(&self) -> bool;
    /*
    **  @brief  set value of user bit
    **  @param  value: bool          the user bit value
    **  @retval none
    */
    fn set_user(&mut self, value: bool);
    /*
    **  @brief  get the execute bit of the entry
    **  @retval bool                 the execute bit
    */
    fn execute(&self) -> bool;
    /*
    **  @brief  set value of user bit
    **  @param  value: bool          the execute bit value
    **  @retval none
    */
    fn set_execute(&mut self, value: bool);
}
