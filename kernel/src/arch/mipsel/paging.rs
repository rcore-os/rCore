// Depends on kernel
use crate::memory::{active_table, alloc_frame, dealloc_frame};
use mips::addr::*;
use mips::tlb::*;
use mips::paging::{Mapper, PageTable as MIPSPageTable, PageTableEntry, PageTableFlags as EF, TwoLevelPageTable};
use mips::paging::{FrameAllocator, FrameDeallocator};
use rcore_memory::paging::*;
use log::*;
#[cfg(target_arch = "riscv32")]
use crate::consts::KERNEL_P2_INDEX;

pub struct ActivePageTable(TwoLevelPageTable<'static>, PageEntry);

/// PageTableEntry: the contents of this entry.
/// Page: this entry is the pte of page `Page`.
pub struct PageEntry(&'static mut PageTableEntry, Page);

impl PageTable for ActivePageTable {

    fn map(&mut self, addr: usize, target: usize) -> &mut Entry {
        // map the 4K `page` to the 4K `frame` with `flags`
        let flags = EF::VALID | EF::WRITABLE | EF::CACHEABLE;
        let page = Page::of_addr(VirtAddr::new(addr));
        let frame = Frame::of_addr(PhysAddr::new(target));
        // map the page to the frame using FrameAllocatorForRiscv
        // we may need frame allocator to alloc frame for new page table(first/second)
        self.0.map_to(page, frame, flags, &mut FrameAllocatorForRiscv).unwrap().flush();
        self.get_entry(addr).expect("fail to get entry")
    }

    fn unmap(&mut self, addr: usize) {
        let page = Page::of_addr(VirtAddr::new(addr));
        let (_, flush) = self.0.unmap(page).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, vaddr: usize) -> Option<&mut Entry> {
        let page = Page::of_addr(VirtAddr::new(vaddr));
        if let Ok(e) = self.0.ref_entry(page.clone()) {
            let e = unsafe { &mut *(e as *mut PageTableEntry) };
            self.1 = PageEntry(e, page);
            Some(&mut self.1 as &mut Entry)
        } else {
            None
        }
    }
}

impl PageTableExt for ActivePageTable {}

/// The virtual address of root page table
extern {
    static root_page_table_buffer : *mut MIPSPageTable;
    static root_page_table_ptr : *mut usize;
}

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(
            TwoLevelPageTable::new(&mut *(root_page_table_buffer as *mut MIPSPageTable)),
            ::core::mem::uninitialized()
        )
    }
}

/// implementation for the Entry trait in /crate/memory/src/paging/mod.rs
impl Entry for PageEntry {
    fn update(&mut self) {
        unsafe { clear_all_tlb(); }
    }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    fn dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITABLE) }
    fn present(&self) -> bool { self.0.flags().contains(EF::VALID) }
    fn clear_accessed(&mut self) { self.0.flags_mut().remove(EF::ACCESSED); }
    fn clear_dirty(&mut self) { self.0.flags_mut().remove(EF::DIRTY); }
    fn set_writable(&mut self, value: bool) { self.0.flags_mut().set(EF::WRITABLE, value); }
    fn set_present(&mut self, value: bool) { self.0.flags_mut().set(EF::VALID, value); }
    fn target(&self) -> usize { self.0.addr().as_usize() }
    fn set_target(&mut self, target: usize) {
        let flags = self.0.flags();
        let frame = Frame::of_addr(PhysAddr::new(target));
        self.0.set(frame, flags);
    }
    fn writable_shared(&self) -> bool { false }
    fn readonly_shared(&self) -> bool { false }
    fn set_shared(&mut self, writable: bool) { }
    fn clear_shared(&mut self) { }
    fn swapped(&self) -> bool { self.0.flags().contains(EF::RESERVED1) }
    fn set_swapped(&mut self, value: bool) { self.0.flags_mut().set(EF::RESERVED1, value); }
    fn user(&self) -> bool { true }
    fn set_user(&mut self, value: bool) { }
    fn execute(&self) -> bool { true }
    fn set_execute(&mut self, value: bool) { }
    fn mmio(&self) -> u8 { 0 }
    fn set_mmio(&mut self, _value: u8) { }
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
        active_table().with_temporary_map(target, |_, table: &mut MIPSPageTable| {
            table.zero();
        });
        InactivePageTable0 { root_frame: frame }
    }

    fn map_kernel(&mut self) { /* nothing to do */ }

    fn token(&self) -> usize {
        self.root_frame.to_kernel_unmapped().as_usize()
    }

    unsafe fn set_token(token: usize) {
        *root_page_table_ptr = token;
    }

    fn active_token() -> usize {
        unsafe { *root_page_table_ptr }
    }

    fn flush_tlb() {
        unsafe { clear_all_tlb(); }
    }

    fn edit<T>(&mut self, f: impl FnOnce(&mut Self::Active) -> T) -> T {
        let pt: *mut MIPSPageTable = unsafe {
            self.token() as *mut MIPSPageTable
        };

        let mut active = unsafe {
            ActivePageTable(
                TwoLevelPageTable::new(&mut *pt),
                ::core::mem::uninitialized()
            )
        };
        f(&mut active)
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
