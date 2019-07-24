//! Memory management structures

use alloc::{boxed::Box, vec::Vec};
use core::fmt::{Debug, Error, Formatter};
use core::mem::size_of;

use crate::paging::*;

use super::*;

use self::handler::MemoryHandler;

pub mod handler;

/// A continuous memory space when the same attribute
#[derive(Debug, Clone)]
pub struct MemoryArea {
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    attr: MemoryAttr,
    handler: Box<dyn MemoryHandler>,
    name: &'static str,
}

impl MemoryArea {
    /// Test whether a virtual address is in the memory area
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start_addr && addr < self.end_addr
    }
    /// Check the array is within the readable memory.
    /// Return the size of space covered in the area.
    fn check_read_array<S>(&self, ptr: *const S, count: usize) -> usize {
        // page align
        let min_bound = (ptr as usize).max(Page::of_addr(self.start_addr).start_address());
        let max_bound = unsafe { ptr.add(count) as usize }
            .min(Page::of_addr(self.end_addr + PAGE_SIZE - 1).start_address());
        if max_bound >= min_bound {
            max_bound - min_bound
        } else {
            0
        }
    }
    /// Check the array is within the writable memory.
    /// Return the size of space covered in the area.
    fn check_write_array<S>(&self, ptr: *mut S, count: usize) -> usize {
        if self.attr.readonly {
            0
        } else {
            self.check_read_array(ptr, count)
        }
    }
    /// Test whether this area is (page) overlap with area [`start_addr`, `end_addr`)
    pub fn is_overlap_with(&self, start_addr: VirtAddr, end_addr: VirtAddr) -> bool {
        let p0 = Page::of_addr(self.start_addr);
        let p1 = Page::of_addr(self.end_addr - 1) + 1;
        let p2 = Page::of_addr(start_addr);
        let p3 = Page::of_addr(end_addr - 1) + 1;
        !(p1 <= p2 || p0 >= p3)
    }
    /// Map all pages in the area to page table `pt`
    fn map(&self, pt: &mut dyn PageTable) {
        for page in Page::range_of(self.start_addr, self.end_addr) {
            self.handler.map(pt, page.start_address(), &self.attr);
        }
    }
    /// Unmap all pages in the area from page table `pt`
    fn unmap(&self, pt: &mut dyn PageTable) {
        for page in Page::range_of(self.start_addr, self.end_addr) {
            self.handler.unmap(pt, page.start_address());
        }
    }
}

/// The attributes of the memory
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct MemoryAttr {
    user: bool,
    readonly: bool,
    execute: bool,
    mmio: u8,
}

impl MemoryAttr {
    pub fn user(mut self) -> Self {
        self.user = true;
        self
    }
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }
    pub fn writable(mut self) -> Self {
        self.readonly = false;
        self
    }
    pub fn execute(mut self) -> Self {
        self.execute = true;
        self
    }
    pub fn mmio(mut self, value: u8) -> Self {
        self.mmio = value;
        self
    }
    /// Apply the attributes to page table entry, then update it.
    /// NOTE: You may need to set present manually.
    pub fn apply(&self, entry: &mut dyn Entry) {
        entry.set_user(self.user);
        entry.set_writable(!self.readonly);
        entry.set_execute(self.execute);
        entry.set_mmio(self.mmio);
        entry.update();
    }
}

/// A set of memory space with multiple memory areas with associated page table
/// NOTE: Don't remove align(64), or you will fail to run MIPS.
/// Temporary solution for rv64
#[cfg_attr(not(target_arch = "riscv64"), repr(align(64)))]
pub struct MemorySet<T: PageTableExt> {
    areas: Vec<MemoryArea>,
    page_table: T,
}

impl<T: PageTableExt> MemorySet<T> {
    /// Create a new `MemorySet`
    pub fn new() -> Self {
        MemorySet {
            areas: Vec::new(),
            page_table: T::new(),
        }
    }
    /// Create a new `MemorySet` for kernel remap
    pub fn new_bare() -> Self {
        MemorySet {
            areas: Vec::new(),
            page_table: T::new_bare(),
        }
    }
    /// Check the pointer is within the readable memory
    pub unsafe fn check_read_ptr<S>(&self, ptr: *const S) -> VMResult<&'static S> {
        self.check_read_array(ptr, 1).map(|s| &s[0])
    }
    /// Check the pointer is within the writable memory
    pub unsafe fn check_write_ptr<S>(&self, ptr: *mut S) -> VMResult<&'static mut S> {
        self.check_write_array(ptr, 1).map(|s| &mut s[0])
    }
    /// Check the array is within the readable memory
    pub unsafe fn check_read_array<S>(
        &self,
        ptr: *const S,
        count: usize,
    ) -> VMResult<&'static [S]> {
        let mut valid_size = 0;
        for area in self.areas.iter() {
            valid_size += area.check_read_array(ptr, count);
            if valid_size == size_of::<S>() * count {
                return Ok(core::slice::from_raw_parts(ptr, count));
            }
        }
        Err(VMError::InvalidPtr)
    }
    /// Check the array is within the writable memory
    pub unsafe fn check_write_array<S>(
        &self,
        ptr: *mut S,
        count: usize,
    ) -> VMResult<&'static mut [S]> {
        let mut valid_size = 0;
        for area in self.areas.iter() {
            valid_size += area.check_write_array(ptr, count);
            if valid_size == size_of::<S>() * count {
                return Ok(core::slice::from_raw_parts_mut(ptr, count));
            }
        }
        Err(VMError::InvalidPtr)
    }
    /// Find a free area with hint address `addr_hint` and length `len`.
    /// Return the start address of found free area.
    /// Used for mmap.
    pub fn find_free_area(&self, addr_hint: usize, len: usize) -> VirtAddr {
        // brute force:
        // try each area's end address as the start
        core::iter::once(addr_hint)
            .chain(self.areas.iter().map(|area| area.end_addr))
            .map(|addr| (addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)) // round up a page
            .find(|&addr| self.test_free_area(addr, addr + len))
            .expect("failed to find free area ???")
    }
    /// Test if [`start_addr`, `end_addr`) is a free area
    fn test_free_area(&self, start_addr: usize, end_addr: usize) -> bool {
        self.areas
            .iter()
            .find(|area| area.is_overlap_with(start_addr, end_addr))
            .is_none()
    }
    /// Add an area to this set
    pub fn push(
        &mut self,
        mut start_addr: VirtAddr,
        mut end_addr: VirtAddr,
        attr: MemoryAttr,
        handler: impl MemoryHandler,
        name: &'static str,
    ) {
        start_addr = start_addr & !(PAGE_SIZE - 1);
        end_addr = (end_addr + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
        assert!(start_addr < end_addr, "invalid memory area");
        assert!(
            self.test_free_area(start_addr, end_addr),
            "memory area overlap"
        );
        let area = MemoryArea {
            start_addr,
            end_addr,
            attr,
            handler: Box::new(handler),
            name,
        };
        area.map(&mut self.page_table);
        // keep order by start address
        let idx = self
            .areas
            .iter()
            .enumerate()
            .find(|(_, other)| start_addr < other.start_addr)
            .map(|(i, _)| i)
            .unwrap_or(self.areas.len());
        self.areas.insert(idx, area);
    }

    /// Remove the area `[start_addr, end_addr)` from `MemorySet`
    pub fn pop(&mut self, start_addr: VirtAddr, end_addr: VirtAddr) {
        assert!(start_addr <= end_addr, "invalid memory area");
        for i in 0..self.areas.len() {
            if self.areas[i].start_addr == start_addr && self.areas[i].end_addr == end_addr {
                let area = self.areas.remove(i);
                area.unmap(&mut self.page_table);
                return;
            }
        }
        panic!("no memory area found");
    }

    /// Remove the area `[start_addr, end_addr)` from `MemorySet`
    /// and split existed ones when necessary.
    pub fn pop_with_split(&mut self, start_addr: VirtAddr, end_addr: VirtAddr) {
        assert!(start_addr <= end_addr, "invalid memory area");
        let mut i = 0;
        while i < self.areas.len() {
            if self.areas[i].is_overlap_with(start_addr, end_addr) {
                if self.areas[i].start_addr >= start_addr && self.areas[i].end_addr <= end_addr {
                    // subset
                    let area = self.areas.remove(i);
                    area.unmap(&mut self.page_table);
                    i -= 1;
                } else if self.areas[i].start_addr >= start_addr
                    && self.areas[i].start_addr < end_addr
                {
                    // prefix
                    let area = self.areas.remove(i);
                    let dead_area = MemoryArea {
                        start_addr: area.start_addr,
                        end_addr,
                        attr: area.attr,
                        handler: area.handler.box_clone(),
                        name: area.name,
                    };
                    dead_area.unmap(&mut self.page_table);
                    let new_area = MemoryArea {
                        start_addr: end_addr,
                        end_addr: area.end_addr,
                        attr: area.attr,
                        handler: area.handler,
                        name: area.name,
                    };
                    self.areas.insert(i, new_area);
                } else if self.areas[i].end_addr <= end_addr && self.areas[i].end_addr > start_addr
                {
                    // postfix
                    let area = self.areas.remove(i);
                    let dead_area = MemoryArea {
                        start_addr,
                        end_addr: area.end_addr,
                        attr: area.attr,
                        handler: area.handler.box_clone(),
                        name: area.name,
                    };
                    dead_area.unmap(&mut self.page_table);
                    let new_area = MemoryArea {
                        start_addr: area.start_addr,
                        end_addr: start_addr,
                        attr: area.attr,
                        handler: area.handler,
                        name: area.name,
                    };
                    self.areas.insert(i, new_area);
                } else {
                    // superset
                    let area = self.areas.remove(i);
                    let dead_area = MemoryArea {
                        start_addr,
                        end_addr,
                        attr: area.attr,
                        handler: area.handler.box_clone(),
                        name: area.name,
                    };
                    dead_area.unmap(&mut self.page_table);
                    let new_area_left = MemoryArea {
                        start_addr: area.start_addr,
                        end_addr: start_addr,
                        attr: area.attr,
                        handler: area.handler.box_clone(),
                        name: area.name,
                    };
                    self.areas.insert(i, new_area_left);
                    let new_area_right = MemoryArea {
                        start_addr: end_addr,
                        end_addr: area.end_addr,
                        attr: area.attr,
                        handler: area.handler,
                        name: area.name,
                    };
                    self.areas.insert(i + 1, new_area_right);
                    i += 1;
                }
            }
            i += 1;
        }
    }

    /// Get iterator of areas
    pub fn iter(&self) -> impl Iterator<Item = &MemoryArea> {
        self.areas.iter()
    }

    /// Execute function `f` with the associated page table
    pub unsafe fn with(&self, f: impl FnOnce()) {
        self.page_table.with(f);
    }
    /// Activate the associated page table
    pub unsafe fn activate(&self) {
        self.page_table.activate();
    }

    /// Get the token of the associated page table
    pub fn token(&self) -> usize {
        self.page_table.token()
    }

    /// Clear and unmap all areas
    pub fn clear(&mut self) {
        let Self {
            ref mut page_table,
            ref mut areas,
            ..
        } = self;
        for area in areas.iter() {
            area.unmap(page_table);
        }
        areas.clear();
    }

    /// Get physical address of the page of given virtual `addr`
    pub fn translate(&mut self, addr: VirtAddr) -> Option<PhysAddr> {
        self.page_table.get_entry(addr).and_then(|entry| {
            if entry.user() {
                Some(entry.target())
            } else {
                None
            }
        })
    }

    /// Get the reference of inner page table
    pub fn get_page_table_mut(&mut self) -> &mut T {
        &mut self.page_table
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        let area = self.areas.iter().find(|area| area.contains(addr));
        match area {
            Some(area) => area.handler.handle_page_fault(&mut self.page_table, addr),
            None => false,
        }
    }

    pub fn clone(&mut self) -> Self {
        let mut new_page_table = T::new();
        let Self {
            ref mut page_table,
            ref areas,
            ..
        } = self;
        for area in areas.iter() {
            for page in Page::range_of(area.start_addr, area.end_addr) {
                area.handler.clone_map(
                    &mut new_page_table,
                    page_table,
                    page.start_address(),
                    &area.attr,
                );
            }
        }
        MemorySet {
            areas: areas.clone(),
            page_table: new_page_table,
        }
    }
}

impl<T: PageTableExt> Drop for MemorySet<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: PageTableExt> Debug for MemorySet<T> {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_list().entries(self.areas.iter()).finish()
    }
}
