use alloc::vec::Vec;
use core::fmt::{Debug, Error, Formatter};
use super::*;

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
        MemoryArea {
            start_addr,
            end_addr,
            phys_start_addr: None,
            flags,
            name,
        }
    }
    pub fn new_identity(start_addr: VirtAddr, end_addr: VirtAddr, flags: MemoryAttr, name: &'static str) -> Self {
        assert!(start_addr <= end_addr, "invalid memory area");
        MemoryArea {
            start_addr,
            end_addr,
            phys_start_addr: Some(PhysAddr::new(start_addr as u64)),
            flags,
            name,
        }
    }
    pub fn new_kernel(start_addr: VirtAddr, end_addr: VirtAddr, flags: MemoryAttr, name: &'static str) -> Self {
        assert!(start_addr <= end_addr, "invalid memory area");
        MemoryArea {
            start_addr,
            end_addr,
            phys_start_addr: Some(PhysAddr::from_kernel_virtual(start_addr)),
            flags,
            name,
        }
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
    fn map(&self, pt: &mut ActivePageTable) {
        match self.phys_start_addr {
            Some(phys_start) => {
                for page in Page::range_of(self.start_addr, self.end_addr) {
                    let frame = Frame::of_addr(phys_start.get() + page.start_address().as_u64() as usize - self.start_addr);
                    self.flags.apply(pt.map_to(page, frame));
                }
            }
            None => {
                for page in Page::range_of(self.start_addr, self.end_addr) {
                    let frame = alloc_frame();
                    self.flags.apply(pt.map_to(page, frame));
                }
            }
        }
    }
    fn unmap(&self, pt: &mut ActivePageTable) {
        for page in Page::range_of(self.start_addr, self.end_addr) {
            let addr = page.start_address().as_u64() as usize;
            if self.phys_start_addr.is_none() {
                let frame = Frame::of_addr(pt.get_entry(addr).target());
                dealloc_frame(frame);
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
    pub fn hide(mut self) -> Self {
        self.hide = true;
        self
    }
    fn apply(&self, entry: &mut impl Entry) {
        if self.user { entry.set_user(true); }
        if self.readonly { entry.set_writable(false); }
        if self.execute { entry.set_execute(true); }
        if self.hide { entry.set_present(false); }
        if self.user || self.readonly || self.execute || self.hide { entry.update(); }
    }
}

/// 内存空间集合，包含若干段连续空间
/// 对应ucore中 `mm_struct`
pub struct MemorySet {
    areas: Vec<MemoryArea>,
    page_table: InactivePageTable,
    kstack: Option<Stack>,
}

impl MemorySet {
    pub fn new(stack_size_in_pages: usize) -> Self {
        MemorySet {
            areas: Vec::<MemoryArea>::new(),
            page_table: new_page_table_with_kernel(),
            kstack: Some(alloc_stack(stack_size_in_pages)),
        }
    }
    /// Used for remap_kernel() where heap alloc is unavailable
    pub unsafe fn new_from_raw_space(slice: &mut [u8]) -> Self {
        use core::mem::size_of;
        let cap = slice.len() / size_of::<MemoryArea>();
        MemorySet {
            areas: Vec::<MemoryArea>::from_raw_parts(slice.as_ptr() as *mut MemoryArea, 0, cap),
            page_table: new_page_table(),
            kstack: None,
        }
    }
    pub fn find_area(&self, addr: VirtAddr) -> Option<&MemoryArea> {
        self.areas.iter().find(|area| area.contains(addr))
    }
    pub fn push(&mut self, area: MemoryArea) {
        assert!(self.areas.iter()
                    .find(|other| area.is_overlap_with(other))
                    .is_none(), "memory area overlap");

        active_table().with(&mut self.page_table, |mapper| area.map(mapper));

        self.areas.push(area);
    }
    pub fn iter(&self) -> impl Iterator<Item=&MemoryArea> {
        self.areas.iter()
    }
    pub fn with(&self, f: impl FnOnce()) {
        let current = unsafe { InactivePageTable::from_cr3() };
        self.page_table.switch();
        f();
        current.switch();
        use core::mem;
        mem::forget(current);
    }
    pub fn switch(&self) {
        self.page_table.switch();
    }
    pub fn set_kstack(&mut self, stack: Stack) {
        assert!(self.kstack.is_none());
        self.kstack = Some(stack);
    }
    pub fn kstack_top(&self) -> usize {
        self.kstack.as_ref().unwrap().top()
    }
    pub fn clone(&self, stack_size_in_pages: usize) -> Self {
        let mut page_table = new_page_table_with_kernel();
        active_table().with(&mut page_table, |mapper| {
            for area in self.areas.iter() {
                area.map(mapper);
            }
        });
        MemorySet {
            areas: self.areas.clone(),
            page_table,
            kstack: Some(alloc_stack(stack_size_in_pages)),
        }
    }
    /// Only for SMP
    pub fn _page_table_addr(&self) -> PhysAddr {
        use core::mem;
        unsafe { mem::transmute_copy::<_, Frame>(&self.page_table) }.start_address()
    }
}

impl Drop for MemorySet {
    fn drop(&mut self) {
        debug!("MemorySet dropping");
        let Self { ref mut page_table, ref areas, .. } = self;
        active_table().with(page_table, |mapper| {
            for area in areas.iter() {
                area.unmap(mapper);
            }
        })
    }
}

impl Debug for MemorySet {
    fn fmt(&self, f: &mut Formatter) -> Result<(), Error> {
        f.debug_list()
            .entries(self.areas.iter())
            .finish()
    }
}

fn new_page_table() -> InactivePageTable {
    let frame = alloc_frame();
    let mut active_table = active_table();
    InactivePageTable::new(frame, &mut active_table)
}

fn new_page_table_with_kernel() -> InactivePageTable {
    let frame = alloc_frame();
    let mut active_table = active_table();
    let mut page_table = InactivePageTable::new(frame, &mut active_table);
    page_table.map_kernel(&mut active_table);
    page_table
}
