use alloc::vec::Vec;
use core::fmt::{Debug, Error, Formatter};
use super::*;
use paging::*;

pub trait InactivePageTable {
    type Active: PageTable;

    fn new() -> Self;
    fn new_bare() -> Self;
    fn edit(&mut self, f: impl FnOnce(&mut Self::Active));
    unsafe fn activate(&self);
    unsafe fn with(&self, f: impl FnOnce());
    fn token(&self) -> usize;

    fn alloc_frame() -> Option<PhysAddr>;
    fn dealloc_frame(target: PhysAddr);
    fn alloc_stack() -> Stack;
}

/// 一片连续内存空间，有相同的访问权限
/// 对应ucore中 `vma_struct`
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct MemoryArea {
    start_addr: VirtAddr,
    end_addr: VirtAddr,
    phys_start_addr: Option<PhysAddr>,
    flags: MemoryAttr,
    name: &'static str,
}

impl MemoryArea {
    pub fn new(start_addr: VirtAddr, end_addr: VirtAddr, flags: MemoryAttr, name: &'static str) -> Self {
        assert!(start_addr <= end_addr, "invalid memory area");
        MemoryArea { start_addr, end_addr, phys_start_addr: None, flags, name }
    }
    pub fn new_identity(start_addr: VirtAddr, end_addr: VirtAddr, flags: MemoryAttr, name: &'static str) -> Self {
        assert!(start_addr <= end_addr, "invalid memory area");
        MemoryArea { start_addr, end_addr, phys_start_addr: Some(start_addr), flags, name }
    }
    pub fn new_physical(phys_start_addr: PhysAddr, phys_end_addr: PhysAddr, offset: usize, flags: MemoryAttr, name: &'static str) -> Self {
        let start_addr = phys_start_addr + offset;
        let end_addr = phys_end_addr + offset;
        assert!(start_addr <= end_addr, "invalid memory area");
        let phys_start_addr = Some(phys_start_addr);
        MemoryArea { start_addr, end_addr, phys_start_addr, flags, name }
    }
    pub unsafe fn as_slice(&self) -> &[u8] {
        use core::slice;
        slice::from_raw_parts(self.start_addr as *const u8, self.end_addr - self.start_addr)
    }
    pub unsafe fn as_slice_mut(&self) -> &mut [u8] {
        use core::slice;
        slice::from_raw_parts_mut(self.start_addr as *mut u8, self.end_addr - self.start_addr)
    }
    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr >= self.start_addr && addr < self.end_addr
    }
    fn is_overlap_with(&self, other: &MemoryArea) -> bool {
        let p0 = Page::of_addr(self.start_addr);
        let p1 = Page::of_addr(self.end_addr - 1) + 1;
        let p2 = Page::of_addr(other.start_addr);
        let p3 = Page::of_addr(other.end_addr - 1) + 1;
        !(p1 <= p2 || p0 >= p3)
    }
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
                }
            }
        }
    }
    fn unmap<T: InactivePageTable>(&self, pt: &mut T::Active) {
        for page in Page::range_of(self.start_addr, self.end_addr) {
            let addr = page.start_address();
            if self.phys_start_addr.is_none() {
                let target = pt.get_entry(addr).target();
                T::dealloc_frame(target);
            }
            pt.unmap(addr);
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct MemoryAttr {
    user: bool,
    readonly: bool,
    execute: bool,
    mmio: bool,
    hide: bool,
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
    pub fn execute(mut self) -> Self {
        self.execute = true;
        self
    }
    pub fn mmio(mut self) -> Self {
        self.mmio = true;
        self
    }
    pub fn hide(mut self) -> Self {
        self.hide = true;
        self
    }
    fn apply(&self, entry: &mut impl Entry) {
        if self.user { entry.set_user(true); }
        if self.readonly { entry.set_writable(false); }
        if self.execute { entry.set_execute(true); }
        if self.mmio { entry.set_mmio(true); }
        if self.hide { entry.set_present(false); }
        if self.user || self.readonly || self.execute || self.mmio || self.hide { entry.update(); }
    }
}

/// 内存空间集合，包含若干段连续空间
/// 对应ucore中 `mm_struct`
pub struct MemorySet<T: InactivePageTable> {
    areas: Vec<MemoryArea>,
    page_table: T,
    kstack: Stack,
}

impl<T: InactivePageTable> MemorySet<T> {
    pub fn new() -> Self {
        MemorySet {
            areas: Vec::<MemoryArea>::new(),
            page_table: T::new(),
            kstack: T::alloc_stack(),
        }
    }
    /// Used for remap_kernel() where heap alloc is unavailable
    pub unsafe fn new_from_raw_space(slice: &mut [u8], kstack: Stack) -> Self {
        use core::mem::size_of;
        let cap = slice.len() / size_of::<MemoryArea>();
        MemorySet {
            areas: Vec::<MemoryArea>::from_raw_parts(slice.as_ptr() as *mut MemoryArea, 0, cap),
            page_table: T::new_bare(),
            kstack,
        }
    }
    pub fn find_area(&self, addr: VirtAddr) -> Option<&MemoryArea> {
        self.areas.iter().find(|area| area.contains(addr))
    }
    pub fn push(&mut self, area: MemoryArea) {
        assert!(self.areas.iter()
                    .find(|other| area.is_overlap_with(other))
                    .is_none(), "memory area overlap");
        self.page_table.edit(|pt| area.map::<T>(pt));
        self.areas.push(area);
    }
    pub fn iter(&self) -> impl Iterator<Item=&MemoryArea> {
        self.areas.iter()
    }
    pub fn edit(&mut self, f: impl FnOnce(&mut T::Active)) {
        self.page_table.edit(f);
    }
    pub unsafe fn with(&self, f: impl FnOnce()) {
        self.page_table.with(f);
    }
    pub unsafe fn activate(&self) {
        self.page_table.activate();
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
    pub fn kstack_top(&self) -> usize {
        self.kstack.top
    }
    pub fn clear(&mut self) {
        let Self { ref mut page_table, ref mut areas, .. } = self;
        page_table.edit(|pt| {
            for area in areas.iter() {
                area.unmap::<T>(pt);
            }
        });
        areas.clear();
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
        MemorySet {
            areas: self.areas.clone(),
            page_table,
            kstack: T::alloc_stack(),
        }
    }
}

impl<T: InactivePageTable> Drop for MemorySet<T> {
    fn drop(&mut self) {
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

#[derive(Debug)]
pub struct Stack {
    pub top: usize,
    pub bottom: usize,
}
