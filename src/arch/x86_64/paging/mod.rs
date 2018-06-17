use memory::*;
//pub use self::cow::*;
use x86_64::structures::paging::*;
use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::instructions::tlb;
use x86_64::ux::u9;

pub type Frame = PhysFrame;
pub type EntryFlags = PageTableFlags;
pub type ActivePageTable = RecursivePageTable<'static>;

pub use x86_64::structures::paging::{Page, PageRange, Mapper, FrameAllocator, FrameDeallocator, Size4KiB, PageTable};

//mod cow;

const ENTRY_COUNT: usize = 512;

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

pub trait ActiveTableExt {
    fn with(&mut self, table: &mut InactivePageTable, f: impl FnOnce(&mut ActivePageTable));
    fn map_to_(&mut self, page: Page, frame: Frame, flags: EntryFlags);
}

impl ActiveTableExt for ActivePageTable {
    fn with(&mut self, table: &mut InactivePageTable, f: impl FnOnce(&mut ActivePageTable)) {
        with_temporary_map(self, &Cr3::read().0, |active_table, p4_table: &mut PageTable| {
            let backup = p4_table[0o777].clone();

            // overwrite recursive mapping
            p4_table[0o777].set_frame(table.p4_frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();

            // execute f in the new context
            f(active_table);

            // restore recursive mapping to original p4 table
            p4_table[0o777] = backup;
            tlb::flush_all();
        });
    }
    fn map_to_(&mut self, page: Page<Size4KiB>, frame: PhysFrame<Size4KiB>, flags: EntryFlags) {
        self.map_to(page, frame, flags, &mut frame_allocator()).unwrap().flush();

        // Set user bit for p1-p4 entry
        // It's a workaround since x86_64 PageTable do not set user bit.
        if flags.contains(EntryFlags::USER_ACCESSIBLE) {
            let mut addr = page.start_address().as_u64();
            for _ in 0..4 {
                addr = ((addr >> 9) & 0o777_777_777_7770) | 0xffffff80_00000000;
                // set USER_ACCESSIBLE
                unsafe { (*(addr as *mut EntryFlags)).insert(EntryFlags::USER_ACCESSIBLE) };
            }
        }
    }
}

#[derive(Debug)]
pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable) -> InactivePageTable {
        with_temporary_map(active_table, &frame, |_, table: &mut PageTable| {
            table.zero();
            // set up recursive mapping for the table
            table[511].set_frame(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        });
        InactivePageTable { p4_frame: frame }
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

fn with_temporary_map(active_table: &mut ActivePageTable, frame: &Frame, f: impl FnOnce(&mut ActivePageTable, &mut PageTable)) {
    // Create a temporary page
    let page = Page::of_addr(0xcafebabe);
    assert!(active_table.translate_page(page).is_none(), "temporary page is already mapped");
    // Map it to table
    active_table.map_to_(page, frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
    // Call f
    let table = unsafe { &mut *page.start_address().as_mut_ptr() };
    f(active_table, table);
    // Unmap the page
    active_table.unmap(page).unwrap().1.flush();
}