// Depends on kernel
use crate::memory::{alloc_frame, dealloc_frame};
use core::mem::ManuallyDrop;
use mips::addr::*;
use mips::paging::{
    FrameAllocator, FrameDeallocator, Mapper, PageTable as MIPSPageTable, PageTableEntry,
    PageTableFlags as EF, TwoLevelPageTable,
};
use mips::tlb::TLBEntry;
use rcore_memory::paging::*;

pub struct PageTableImpl {
    page_table: TwoLevelPageTable<'static>,
    root_frame: Frame,
    entry: Option<PageEntry>,
}

/// PageTableEntry: the contents of this entry.
/// Page: this entry is the pte of page `Page`.
pub struct PageEntry(&'static mut PageTableEntry, Page);

impl PageTable for PageTableImpl {
    fn map(&mut self, addr: usize, target: usize) -> &mut dyn Entry {
        // map the 4K `page` to the 4K `frame` with `flags`
        let flags = EF::VALID | EF::WRITABLE | EF::CACHEABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target));
        // we may need frame allocator to alloc frame for new page table(first/second)
        self.page_table
            .map_to(page, frame, flags, &mut FrameAllocatorForMips)
            .unwrap()
            .flush();
        self.get_entry(addr).expect("fail to get entry")
    }

    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (_, flush) = self.page_table.unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, vaddr: usize) -> Option<&mut dyn Entry> {
        let page = Page::of_addr(VirtAddr::new(vaddr));
        if let Ok(e) = self.page_table.ref_entry(page.clone()) {
            let e = unsafe { &mut *(e as *mut PageTableEntry) };
            self.entry = Some(PageEntry(e, page));
            Some(self.entry.as_mut().unwrap())
        } else {
            None
        }
    }

    fn get_page_slice_mut<'a>(&mut self, addr: usize) -> &'a mut [u8] {
        let frame = self
            .page_table
            .translate_page(Page::of_addr(VirtAddr::new(addr)))
            .unwrap();
        let vaddr = frame.to_kernel_unmapped().as_usize();
        unsafe { core::slice::from_raw_parts_mut(vaddr as *mut u8, 0x1000) }
    }

    fn flush_cache_copy_user(&mut self, _start: usize, _end: usize, _execute: bool) {}
}

extern "C" {
    fn _root_page_table_buffer();
    fn _root_page_table_ptr();
}

pub fn set_root_page_table_ptr(ptr: usize) {
    unsafe {
        TLBEntry::clear_all();
        *(_root_page_table_ptr as *mut usize) = ptr;
    }
}

pub fn get_root_page_table_ptr() -> usize {
    unsafe { *(_root_page_table_ptr as *mut usize) }
}

pub fn root_page_table_buffer() -> &'static mut MIPSPageTable {
    unsafe { &mut *(_root_page_table_ptr as *mut MIPSPageTable) }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        TLBEntry::clear_all();
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

impl PageTableImpl {
    /// Unsafely get the current active page table.
    /// Using ManuallyDrop to wrap the page table: this is how `core::mem::forget` is implemented now.
    pub unsafe fn active() -> ManuallyDrop<Self> {
        let frame = Frame::of_addr(PhysAddr::new(get_root_page_table_ptr() & 0x7fffffff));
        let table = root_page_table_buffer();
        ManuallyDrop::new(PageTableImpl {
            page_table: TwoLevelPageTable::new(table),
            root_frame: frame,
            entry: None,
        })
    }

    /// The method for getting the kernel page table.
    /// In mipsel kernel page table and user page table are the same table. However you have to do the initialization.
    pub unsafe fn kernel_table() -> ManuallyDrop<Self> {
        Self::active()
    }
}

impl PageTableExt for PageTableImpl {
    fn new_bare() -> Self {
        let target = alloc_frame().expect("failed to allocate frame");
        let frame = Frame::of_addr(PhysAddr::new(target));

        let table = unsafe { &mut *(target as *mut MIPSPageTable) };
        table.zero();

        PageTableImpl {
            page_table: TwoLevelPageTable::new(table),
            root_frame: frame,
            entry: None,
        }
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
        TLBEntry::clear_all();
    }
}

impl Drop for PageTableImpl {
    fn drop(&mut self) {
        dealloc_frame(self.root_frame.start_address().as_usize());
    }
}

struct FrameAllocatorForMips;

impl FrameAllocator for FrameAllocatorForMips {
    fn alloc(&mut self) -> Option<Frame> {
        alloc_frame().map(|addr| Frame::of_addr(PhysAddr::new(addr)))
    }
}

impl FrameDeallocator for FrameAllocatorForMips {
    fn dealloc(&mut self, frame: Frame) {
        dealloc_frame(frame.start_address().as_usize());
    }
}
