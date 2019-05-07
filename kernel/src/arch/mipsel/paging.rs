// Depends on kernel
use crate::memory::{active_table, alloc_frame, dealloc_frame};
use log::*;
use mips::addr::*;
use mips::paging::{FrameAllocator, FrameDeallocator};
use mips::paging::{
    Mapper, PageTable as MIPSPageTable, PageTableEntry, PageTableFlags as EF, TwoLevelPageTable,
};
use mips::tlb::*;
use rcore_memory::paging::*;

pub struct ActivePageTable(usize, PageEntry);

/// PageTableEntry: the contents of this entry.
/// Page: this entry is the pte of page `Page`.
pub struct PageEntry(&'static mut PageTableEntry, Page);

impl PageTable for ActivePageTable {
    fn map(&mut self, addr: usize, target: usize) -> &mut Entry {
        // map the 4K `page` to the 4K `frame` with `flags`
        let flags = EF::VALID | EF::WRITABLE | EF::CACHEABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target));
        // we may need frame allocator to alloc frame for new page table(first/second)
        self.get_table()
            .map_to(page, frame, flags, &mut FrameAllocatorForRiscv)
            .unwrap()
            .flush();
        self.get_entry(addr).expect("fail to get entry")
    }

    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (_, flush) = self.get_table().unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, vaddr: usize) -> Option<&mut Entry> {
        let page = Page::of_addr(VirtAddr::new(vaddr));
        if let Ok(e) = self.get_table().ref_entry(page.clone()) {
            let e = unsafe { &mut *(e as *mut PageTableEntry) };
            self.1 = PageEntry(e, page);
            Some(&mut self.1 as &mut Entry)
        } else {
            None
        }
    }
}

extern "C" {
    fn _root_page_table_buffer();
    fn _root_page_table_ptr();
}

pub fn set_root_page_table_ptr(ptr: usize) {
    unsafe {
        clear_all_tlb();
        *(_root_page_table_ptr as *mut usize) = ptr;
    }
}

pub fn get_root_page_table_ptr() -> usize {
    unsafe { *(_root_page_table_ptr as *mut usize) }
}

pub fn root_page_table_buffer() -> &'static mut MIPSPageTable {
    unsafe { &mut *(_root_page_table_ptr as *mut MIPSPageTable) }
}

impl PageTableExt for ActivePageTable {}

static mut __page_table_with_mode: bool = false;

/// The virtual address of root page table

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(
            get_root_page_table_ptr(),
            ::core::mem::uninitialized(),
        )
    }

    unsafe fn get_raw_table(&mut self) -> *mut MIPSPageTable {
        if __page_table_with_mode {
            get_root_page_table_ptr() as *mut MIPSPageTable
        } else {
            self.0 as *mut MIPSPageTable
        }
    }

    fn get_table(&mut self) -> TwoLevelPageTable<'static> {
        unsafe {
            TwoLevelPageTable::new(&mut *self.get_raw_table())
        }
    }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        unsafe {
            clear_all_tlb();
        }
    }
    fn accessed(&self) -> bool {
        self.0.flags().contains(EF::ACCESSED)
    }
    fn dirty(&self) -> bool {
        self.0.flags().contains(EF::DIRTY)
    }
    fn writable(&self) -> bool {
        self.0.flags().contains(EF::WRITABLE)
    }
    fn present(&self) -> bool {
        self.0.flags().contains(EF::VALID)
    }
    fn clear_accessed(&mut self) {
        self.0.flags_mut().remove(EF::ACCESSED);
    }
    fn clear_dirty(&mut self) {
        self.0.flags_mut().remove(EF::DIRTY);
    }
    fn set_writable(&mut self, value: bool) {
        self.0.flags_mut().set(EF::WRITABLE, value);
    }
    fn set_present(&mut self, value: bool) {
        self.0.flags_mut().set(EF::VALID, value);
    }
    fn target(&self) -> usize {
        self.0.addr().as_usize()
    }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target));
        self.0.set(frame, flags);
    }
    fn writable_shared(&self) -> bool {
        false
    }
    fn readonly_shared(&self) -> bool {
        false
    }
    fn set_shared(&mut self, writable: bool) {}
    fn clear_shared(&mut self) {}
    fn swapped(&self) -> bool {
        self.0.flags().contains(EF::RESERVED1)
    }
    fn set_swapped(&mut self, value: bool) {
        self.0.flags_mut().set(EF::RESERVED1, value);
    }
    fn user(&self) -> bool {
        true
    }
    fn set_user(&mut self, value: bool) {}
    fn execute(&self) -> bool {
        true
    }
    fn set_execute(&mut self, value: bool) {}
    fn mmio(&self) -> u8 {
        0
    }
    fn set_mmio(&mut self, _value: u8) {}
}

#[derive(Debug)]
pub struct InactivePageTable0 {
    root_frame: Frame,
}

impl InactivePageTable for InactivePageTable0 {
    type Active = ActivePageTable;

    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(PhysAddr::new(target));

        let table = unsafe { &mut *(target as *mut MIPSPageTable) };

        table.zero();
        InactivePageTable0 { root_frame: frame }
    }

    fn map_kernel(&mut self) {
        /* nothing to do */
    }

    fn token(&self) -> usize {
        self.root_frame.to_kernel_unmapped().as_usize()
    }

    unsafe fn set_token(token: usize) {
        set_root_page_table_ptr(token);
    }

    fn active_token() -> usize {
        get_root_page_table_ptr()
    }

    fn flush_tlb() {
        unsafe {
            clear_all_tlb();
        }
    }

    fn edit<T>(&mut self, f: impl FnOnce(&mut Self::Active) -> T) -> T {
        unsafe {
            clear_all_tlb();
        }

        debug!("edit table {:x?} -> {:x?}", Self::active_token(), self.token());
        let mut active = unsafe {
            ActivePageTable(
                self.token(),
                ::core::mem::uninitialized(),
            )
        };

        let ret = f(&mut active);
        debug!("finish table");

        unsafe {
            clear_all_tlb();
        }
        ret
    }

    unsafe fn with<T>(&self, f: impl FnOnce() -> T) -> T {
        let old_token = Self::active_token();
        let new_token = self.token();

        let old_mode = unsafe { __page_table_with_mode };
        unsafe { 
            __page_table_with_mode = true;
        }

        debug!("switch table {:x?} -> {:x?}", old_token, new_token);
        if old_token != new_token {
            Self::set_token(new_token);
            Self::flush_tlb();
        }
        let ret = f();
        debug!("switch table {:x?} -> {:x?}", new_token, old_token);
        if old_token != new_token {
            Self::set_token(old_token);
            Self::flush_tlb();
        }

        unsafe {
            __page_table_with_mode = old_mode;
        }

        ret
    }
}

impl Drop for InactivePageTable0 {
    fn drop(&mut self) {
        dealloc_frame(self.root_frame.start_address().as_usize());
    }
}

struct FrameAllocatorForRiscv;

impl FrameAllocator for FrameAllocatorForRiscv {
    fn alloc(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(PhysAddr::new(addr)))
    }
}

impl FrameDeallocator for FrameAllocatorForRiscv {
    fn dealloc(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_usize());
    }
}
