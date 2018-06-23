use memory::*;
pub use ucore_memory::paging::{Entry, PageTable};
use x86_64::instructions::tlb;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{Mapper, PageTable as x86PageTable, PageTableEntry, PageTableFlags as EF, RecursivePageTable};
pub use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, Page, PageRange, PhysFrame as Frame, Size4KiB};
use x86_64::ux::u9;

pub trait PageExt {
    fn of_addr(address: VirtAddr) -> Self;
    fn range_of(begin: VirtAddr, end: VirtAddr) -> PageRange;
}

impl PageExt for Page {
    fn of_addr(address: usize) -> Self {
        use x86_64;
        Page::containing_address(x86_64::VirtAddr::new(address as u64))
    }
    fn range_of(begin: usize, end: usize) -> PageRange<Size4KiB> {
        Page::range(Page::of_addr(begin), Page::of_addr(end - 1) + 1)
    }
}

pub trait FrameExt {
    fn of_addr(address: usize) -> Self;
}

impl FrameExt for Frame {
    fn of_addr(address: usize) -> Self {
        Frame::containing_address(PhysAddr::new(address as u64))
    }
}

pub struct ActivePageTable(RecursivePageTable<'static>);

pub struct PageEntry(PageTableEntry);

impl PageTable for ActivePageTable {
    type Entry = PageEntry;

    fn map(&mut self, addr: usize, target: usize) -> &mut PageEntry {
        let flags = EF::PRESENT | EF::WRITABLE | EF::NO_EXECUTE;
        self.0.map_to(Page::of_addr(addr), Frame::of_addr(target), flags, &mut frame_allocator())
            .unwrap().flush();
        self.get_entry(addr)
    }

    fn unmap(&mut self, addr: usize) {
        let (frame, flush) = self.0.unmap(Page::of_addr(addr)).unwrap();
        flush.flush();
    }

    fn get_entry(&mut self, addr: usize) -> &mut PageEntry {
        let entry_addr = ((addr >> 9) & 0o777_777_777_7770) | 0xffffff80_00000000;
        unsafe { &mut *(entry_addr as *mut PageEntry) }
    }

    fn read_page(&mut self, addr: usize, data: &mut [u8]) {
        use core::slice;
        let mem = unsafe { slice::from_raw_parts((addr & !0xfffusize) as *const u8, 4096) };
        data.copy_from_slice(mem);
    }

    fn write_page(&mut self, addr: usize, data: &[u8]) {
        use core::slice;
        let mem = unsafe { slice::from_raw_parts_mut((addr & !0xfffusize) as *mut u8, 4096) };
        mem.copy_from_slice(data);
    }
}

impl ActivePageTable {
    pub unsafe fn new() -> Self {
        ActivePageTable(RecursivePageTable::new(&mut *(0xffffffff_fffff000 as *mut _)).unwrap())
    }
    pub fn with(&mut self, table: &mut InactivePageTable, f: impl FnOnce(&mut ActivePageTable)) {
        with_temporary_map(self, &Cr3::read().0, |active_table, p4_table: &mut x86PageTable| {
            let backup = p4_table[0o777].clone();

            // overwrite recursive mapping
            p4_table[0o777].set_frame(table.p4_frame.clone(), EF::PRESENT | EF::WRITABLE);
            tlb::flush_all();

            // execute f in the new context
            f(active_table);

            // restore recursive mapping to original p4 table
            p4_table[0o777] = backup;
            tlb::flush_all();
        });
    }
    pub fn map_to(&mut self, page: Page, frame: Frame) -> &mut PageEntry {
        self.map(page.start_address().as_u64() as usize, frame.start_address().as_u64() as usize)
    }
}

impl Entry for PageEntry {
    fn update(&mut self) {
        use x86_64::{VirtAddr, instructions::tlb::flush};
        let addr = VirtAddr::new_unchecked((self as *const _ as u64) << 9);
        flush(addr);
    }
    fn accessed(&self) -> bool { self.0.flags().contains(EF::ACCESSED) }
    fn dirty(&self) -> bool { self.0.flags().contains(EF::DIRTY) }
    fn writable(&self) -> bool { self.0.flags().contains(EF::WRITABLE) }
    fn present(&self) -> bool { self.0.flags().contains(EF::PRESENT) }
    fn clear_accessed(&mut self) { self.as_flags().remove(EF::ACCESSED); }
    fn clear_dirty(&mut self) { self.as_flags().remove(EF::DIRTY); }
    fn set_writable(&mut self, value: bool) { self.as_flags().set(EF::WRITABLE, value); }
    fn set_present(&mut self, value: bool) { self.as_flags().set(EF::PRESENT, value); }
    fn target(&self) -> usize { self.0.addr().as_u64() as usize }
    fn writable_shared(&self) -> bool { self.0.flags().contains(EF::BIT_10) }
    fn readonly_shared(&self) -> bool { self.0.flags().contains(EF::BIT_9) }
    fn set_shared(&mut self, writable: bool) {
        let flags = self.as_flags();
        flags.set(EF::BIT_10, writable);
        flags.set(EF::BIT_9, !writable);
    }
    fn clear_shared(&mut self) { self.as_flags().remove(EF::BIT_9 | EF::BIT_10); }
    fn user(&self) -> bool { self.0.flags().contains(EF::USER_ACCESSIBLE) }
    fn set_user(&mut self, value: bool) {
        self.as_flags().set(EF::USER_ACCESSIBLE, value);
        if value {
            let mut addr = self as *const _ as usize;
            for _ in 0..3 {
                // Upper level entry
                addr = ((addr >> 9) & 0o777_777_777_7770) | 0xffffff80_00000000;
                // set USER_ACCESSIBLE
                unsafe { (*(addr as *mut EF)).insert(EF::USER_ACCESSIBLE) };
            }
        }
    }
    fn execute(&self) -> bool { !self.0.flags().contains(EF::NO_EXECUTE) }
    fn set_execute(&mut self, value: bool) { self.as_flags().set(EF::NO_EXECUTE, !value); }
}

impl PageEntry {
    fn as_flags(&mut self) -> &mut EF {
        unsafe { &mut *(self as *mut _ as *mut EF) }
    }
}

#[derive(Debug)]
pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable) -> InactivePageTable {
        with_temporary_map(active_table, &frame, |_, table: &mut x86PageTable| {
            table.zero();
            // set up recursive mapping for the table
            table[511].set_frame(frame.clone(), EF::PRESENT | EF::WRITABLE);
        });
        InactivePageTable { p4_frame: frame }
    }
    pub fn map_kernel(&mut self, active_table: &mut ActivePageTable) {
        let mut table = unsafe { &mut *(0xffffffff_fffff000 as *mut x86PageTable) };
        let e510 = table[510].clone();
        let e509 = table[509].clone();

        active_table.with(self, |pt: &mut ActivePageTable| {
            table[510] = e510;
            table[509] = e509;
        });
    }
    pub fn switch(&self) {
        let old_frame = Cr3::read().0;
        let new_frame = self.p4_frame.clone();
        debug!("switch table {:?} -> {:?}", old_frame, new_frame);
        if old_frame != new_frame {
            unsafe { Cr3::write(new_frame, Cr3Flags::empty()); }
        }
    }
    pub unsafe fn from_cr3() -> Self {
        InactivePageTable { p4_frame: Cr3::read().0 }
    }
}

impl Drop for InactivePageTable {
    fn drop(&mut self) {
        info!("PageTable dropping: {:?}", self);
        dealloc_frame(self.p4_frame.clone());
    }
}

fn with_temporary_map(active_table: &mut ActivePageTable, frame: &Frame, f: impl FnOnce(&mut ActivePageTable, &mut x86PageTable)) {
    // Create a temporary page
    let page = Page::of_addr(0xcafebabe);
    assert!(active_table.0.translate_page(page).is_none(), "temporary page is already mapped");
    // Map it to table
    active_table.map_to(page, frame.clone());
    // Call f
    let table = unsafe { &mut *page.start_address().as_mut_ptr() };
    f(active_table, table);
    // Unmap the page
    active_table.unmap(0xcafebabe);
}