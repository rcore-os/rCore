use super::{Page, ActivePageTable};
use super::table::{Table, Level1};
use memory::{Frame, FrameAllocator, VirtAddr};

pub struct TemporaryPage {
    page: Page,
}

impl TemporaryPage {
    pub fn new(page: Page) -> TemporaryPage {
        TemporaryPage { page }
    }

    /// Maps the temporary page to the given frame in the active table.
    /// Returns the start address of the temporary page.
    pub fn map(&self, frame: Frame, active_table: &mut ActivePageTable)
        -> VirtAddr
    {
        use super::entry::EntryFlags;

        assert!(active_table.translate_page(self.page).is_none(),
                "temporary page is already mapped");
        active_table.map_to(self.page, frame, EntryFlags::WRITABLE);
        self.page.start_address()
    }

    /// Unmaps the temporary page in the active table.
    pub fn unmap(&self, active_table: &mut ActivePageTable) {
        active_table.unmap(self.page)
    }

    /// Maps the temporary page to the given page table frame in the active
    /// table. Returns a reference to the now mapped table.
    pub fn map_table_frame(&self,
                        frame: Frame,
                        active_table: &mut ActivePageTable)
                        -> &mut Table<Level1> {
        unsafe { &mut *(self.map(frame, active_table) as *mut Table<Level1>) }
    }
}
