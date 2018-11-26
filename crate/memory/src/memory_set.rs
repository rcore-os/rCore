//! memory set, area
//! and the inactive page table

use alloc::vec::Vec;
use core::fmt::{Debug, Error, Formatter};
use super::*;
use crate::paging::*;

/// an inactive page table
/// Note: InactivePageTable is not a PageTable
///       but it can be activated and "become" a PageTable
/// Why this trait is in this file?(seems should in paging/mod.rs)
pub trait InactivePageTable {
    /// the active version of page table
    type Active: PageTable;

    /*
    **  @brief  create a inactive page table with kernel memory mapped
    **  @retval InactivePageTable    the created inactive page table
    */
    fn new() -> Self;
    /*
    **  @brief  create an inactive page table without kernel memory mapped
    **  @retval InactivePageTable    the created inactive page table
    */
    fn new_bare() -> Self;
    /*
    **  @brief  temporarily active the page table and edit it
    **  @retval impl FnOnce(&mut Self::Active)
    **                               the function of the editing action,
    **                               which takes a temporarily activated page table as param
    **  @retval none
    */
    fn edit(&mut self, f: impl FnOnce(&mut Self::Active));
    /*
    **  @brief  activate the inactive page table
    **  @retval none
    */
    unsafe fn activate(&self);
    /*
    **  @brief  execute function with this inactive page table
    **  @param  f: impl FnOnce()     the function to be executed
    **  @retval none
    */
    unsafe fn with<T>(&self, f: impl FnOnce() -> T) -> T;
    /*
    **  @brief  get the token of the inactive page table
    **  @retval usize                the token of the inactive page table
    */
    fn token(&self) -> usize;

    /// Why the methods below are in this trait?
    /*
    **  @brief  allocate a frame for use
    **  @retval Option<PhysAddr>     the physics address of the beginning of allocated frame, if present
    */
    fn alloc_frame() -> Option<PhysAddr>;
    /*
    **  @brief  deallocate a frame for use
    **  @param  PhysAddr             the physics address of the beginning of frame to be deallocated
    **  @retval none
    */
    fn dealloc_frame(target: PhysAddr);
}

/// a continuous memory space when the same attribute
/// like `vma_struct` in ucore
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct MemoryArea {
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    phys_start_addr: Option<PhysAddr>,
    flags: MemoryAttr,
    name: &'static str,
}

impl MemoryArea {
    /*
    **  @brief  create a memory area from virtual address
    **  @param  start_addr: VirtAddr the virtual address of beginning of the area
    **  @param  end_addr: VirtAddr   the virtual address of end of the area
    **  @param  flags: MemoryAttr    the common memory attribute of the memory area
    **  @param  name: &'static str   the name of the memory area
    **  @retval MemoryArea           the memory area created
    */
    pub fn new(start_addr: VirtAddr, end_addr: VirtAddr, flags: MemoryAttr, name: &'static str) -> Self {
        assert!(start_addr <= end_addr, "invalid memory area");
        MemoryArea { start_addr, end_addr, phys_start_addr: None, flags, name }
    }
    /*
    **  @brief  create a memory area from virtual address which is identically mapped
    **  @param  start_addr: VirtAddr the virtual address of beginning of the area
    **  @param  end_addr: VirtAddr   the virtual address of end of the area
    **  @param  flags: MemoryAttr    the common memory attribute of the memory area
    **  @param  name: &'static str   the name of the memory area
    **  @retval MemoryArea           the memory area created
    */
    pub fn new_identity(start_addr: VirtAddr, end_addr: VirtAddr, flags: MemoryAttr, name: &'static str) -> Self {
        assert!(start_addr <= end_addr, "invalid memory area");
        MemoryArea { start_addr, end_addr, phys_start_addr: Some(start_addr), flags, name }
    }
    /*
    **  @brief  create a memory area from physics address
    **  @param  start_addr: PhysAddr the physics address of beginning of the area
    **  @param  end_addr: PhysAddr   the physics address of end of the area
    **  @param  offset: usiz         the offset between physics address and virtual address
    **  @param  flags: MemoryAttr    the common memory attribute of the memory area
    **  @param  name: &'static str   the name of the memory area
    **  @retval MemoryArea           the memory area created
    */
    pub fn new_physical(phys_start_addr: PhysAddr, phys_end_addr: PhysAddr, offset: usize, flags: MemoryAttr, name: &'static str) -> Self {
        let start_addr = phys_start_addr + offset;
        let end_addr = phys_end_addr + offset;
        assert!(start_addr <= end_addr, "invalid memory area");
        let phys_start_addr = Some(phys_start_addr);
        MemoryArea { start_addr, end_addr, phys_start_addr, flags, name }
    }
    /*
    **  @brief  get slice of the content in the memory area
    **  @retval &[u8]                the slice of the content in the memory area
    */
    pub unsafe fn as_slice(&self) -> &[u8] {
        use core::slice;
        slice::from_raw_parts(self.start_addr as *const u8, self.end_addr - self.start_addr)
    }
    /*
    **  @brief  get mutable slice of the content in the memory area
    **  @retval &mut[u8]             the mutable slice of the content in the memory area
    */
    pub unsafe fn as_slice_mut(&self) -> &mut [u8] {
        use core::slice;
        slice::from_raw_parts_mut(self.start_addr as *mut u8, self.end_addr - self.start_addr)
    }
    /*
    **  @brief  test whether a virtual address is in the memory area
    **  @param  addr: VirtAddr       the virtual address to test
    **  @retval bool                 whether the virtual address is in the memory area
    */
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start_addr && addr < self.end_addr
    }
    /*
    **  @brief  test whether the memory area is overlap with another memory area
    **  @param  other: &MemoryArea   another memory area to test
    **  @retval bool                 whether the memory area is overlap with another memory area
    */
    fn is_overlap_with(&self, other: &MemoryArea) -> bool {
        let p0 = Page::of_addr(self.start_addr);
        let p1 = Page::of_addr(self.end_addr - 1) + 1;
        let p2 = Page::of_addr(other.start_addr);
        let p3 = Page::of_addr(other.end_addr - 1) + 1;
        !(p1 <= p2 || p0 >= p3)
    }
    /*
    **  @brief  map the memory area to the physice address in a page table
    **  @param  pt: &mut T::Active   the page table to use
    **  @retval none
    */
    fn map<T: InactivePageTable>(&self, pt: &mut T::Active) {
        match self.phys_start_addr {
            Some(phys_start) => {
                for page in Page::range_of(self.start_addr, self.end_addr) {
                    let addr = page.start_address();
                    let target = page.start_address() - self.start_addr + phys_start;
                    self.flags.apply(pt.map(addr, target));
                }
            }
            None => {
                for page in Page::range_of(self.start_addr, self.end_addr) {
                    let addr = page.start_address();
                    let target = T::alloc_frame().expect("failed to allocate frame");
                    self.flags.apply(pt.map(addr, target));
                    // for frame delayed allocation
//                    let entry = pt.map(addr,0);
//                    self.flags.apply(entry);
//                    let entry = pt.get_entry(addr).expect("fail to get entry");
//                    entry.set_present(false);
//                    entry.update();
                }
            }
        };
    }
    /*
    **  @brief  unmap the memory area from the physice address in a page table
    **  @param  pt: &mut T::Active   the page table to use
    **  @retval none
    */
    fn unmap<T: InactivePageTable>(&self, pt: &mut T::Active) {
        for page in Page::range_of(self.start_addr, self.end_addr) {
            let addr = page.start_address();
            if self.phys_start_addr.is_none() {
                if pt.get_entry(addr).expect("fail to get entry").present(){
                    let target = pt.get_entry(addr).expect("fail to get entry").target();
                    T::dealloc_frame(target);
                }
                else{
                    // set valid for pt.unmap function
                    pt.get_entry(addr).expect("fail to get entry").set_present(true);
                }
            }
            pt.unmap(addr);
        }
    }

    pub fn get_start_addr(&self) -> VirtAddr {
        self.start_addr
    }

    pub fn get_end_addr(&self) -> VirtAddr{
        self.end_addr
    }

    pub fn get_flags(&self) -> &MemoryAttr{
        &self.flags
    }

}

/// The attributes of the memory
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct MemoryAttr {
    user: bool,
    readonly: bool,
    execute: bool,
    hide: bool,
}

impl MemoryAttr {
    /*
    **  @brief  set the memory attribute's user bit
    **  @retval MemoryAttr           the memory attribute itself
    */
    pub fn user(mut self) -> Self {
        self.user = true;
        self
    }
    /*
    **  @brief  set the memory attribute's readonly bit
    **  @retval MemoryAttr           the memory attribute itself
    */
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }
    /*
    **  @brief  set the memory attribute's execute bit
    **  @retval MemoryAttr           the memory attribute itself
    */
    pub fn execute(mut self) -> Self {
        self.execute = true;
        self
    }
    /*
    **  @brief  set the memory attribute's hide bit
    **  @retval MemoryAttr           the memory attribute itself
    */
    pub fn hide(mut self) -> Self {
        self.hide = true;
        self
    }
    /*
    **  @brief  apply the memory attribute to a page table entry
    **  @param  entry: &mut impl Entry
    **                               the page table entry to apply the attribute
    **  @retval none
    */
    fn apply(&self, entry: &mut impl Entry) {
        if self.user { entry.set_user(true); }
        if self.readonly { entry.set_writable(false); }
        if self.execute { entry.set_execute(true); }
        if self.hide { entry.set_present(false); }
        if self.user || self.readonly || self.execute || self.hide { entry.update(); }
    }
}

/// set of memory space with multiple memory area with associated page table and stack space
/// like `mm_struct` in ucore
pub struct MemorySet<T: InactivePageTable> {
    areas: Vec<MemoryArea>,
    page_table: T,
}

impl<T: InactivePageTable> MemorySet<T> {
    /*
    **  @brief  create a memory set
    **  @retval MemorySet<T>         the memory set created
    */
    pub fn new() -> Self {
        MemorySet {
            areas: Vec::<MemoryArea>::new(),
            page_table: T::new(),
        }
    }
    pub fn new_bare() -> Self {
        MemorySet {
            areas: Vec::<MemoryArea>::new(),
            page_table: T::new_bare(),
        }
    }
    /*
    **  @brief  find the memory area from virtual address
    **  @param  addr: VirtAddr       the virtual address to find
    **  @retval Option<&MemoryArea>  the memory area with the virtual address, if presented
    */
    pub fn find_area(&self, addr: VirtAddr) -> Option<&MemoryArea> {
        self.areas.iter().find(|area| area.contains(addr))
    }
    /*
    **  @brief  add the memory area to the memory set
    **  @param  area: MemoryArea     the memory area to add
    **  @retval none
    */
    pub fn push(&mut self, area: MemoryArea) {
        assert!(self.areas.iter()
                    .find(|other| area.is_overlap_with(other))
                    .is_none(), "memory area overlap");
        self.page_table.edit(|pt| area.map::<T>(pt));
        self.areas.push(area);
    }
    /*
    **  @brief  get iterator of the memory area
    **  @retval impl Iterator<Item=&MemoryArea>
    **                               the memory area iterator
    */
    pub fn iter(&self) -> impl Iterator<Item=&MemoryArea> {
        self.areas.iter()
    }
    /*
    **  @brief  execute function with the associated page table
    **  @param  f: impl FnOnce()     the function to be executed
    **  @retval none
    */
    pub unsafe fn with(&self, f: impl FnOnce()) {
        self.page_table.with(f);
    }
    /*
    **  @brief  activate the associated page table
    **  @retval none
    */
    pub unsafe fn activate(&self) {
        self.page_table.activate();
    }
    /*
    **  @brief  get the token of the associated page table
    **  @retval usize                the token of the inactive page table
    */
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
    /*
    **  @brief  clear the memory set
    **  @retval none
    */
    pub fn clear(&mut self) {
        let Self { ref mut page_table, ref mut areas, .. } = self;
        page_table.edit(|pt| {
            for area in areas.iter() {
                area.unmap::<T>(pt);
            }
        });
        areas.clear();
    }

    /*
    **  @brief  get the mutable reference for the inactive page table
    **  @retval: &mut T                 the mutable reference of the inactive page table
    */
    pub fn get_page_table_mut(&mut self) -> &mut T{
        &mut self.page_table
    }

}

impl<T: InactivePageTable> Clone for MemorySet<T> {
    fn clone(&self) -> Self {
        let mut page_table = T::new();
        page_table.edit(|pt| {
            for area in self.areas.iter() {
                area.map::<T>(pt);
            }
        });
        info!("finish map in clone!");
        MemorySet {
            areas: self.areas.clone(),
            page_table,
        }
    }
}

impl<T: InactivePageTable> Drop for MemorySet<T> {
    fn drop(&mut self) {
        info!("come into drop func for memoryset");
        self.clear();
    }
}

impl<T: InactivePageTable> Debug for MemorySet<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_list()
            .entries(self.areas.iter())
            .finish()
    }
}
