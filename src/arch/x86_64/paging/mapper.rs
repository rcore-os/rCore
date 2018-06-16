use core::ptr::Unique;
use memory::*;
use super::{ENTRY_COUNT, EntryFlags, Page};
use super::table::{self, Level1, Level4, Table};

pub struct Mapper {
    p4: Unique<Table<Level4>>,
}

impl Mapper {
    pub const unsafe fn new() -> Mapper {
        Mapper {
            p4: Unique::new_unchecked(table::P4),
        }
    }

    pub fn p4(&self) -> &Table<Level4> {
        unsafe { self.p4.as_ref() }
    }

    pub fn p4_mut(&mut self) -> &mut Table<Level4> {
        unsafe { self.p4.as_mut() }
    }

    pub fn translate(&self, virtual_address: VirtAddr) -> Option<PhysAddr> {
        let offset = virtual_address % PAGE_SIZE;
        self.translate_page(Page::of_addr(virtual_address))
            .map(|frame| PhysAddr::new((frame.start_address().get() + offset) as u64))
    }

    pub fn translate_page(&self, page: Page) -> Option<Frame> {
        let p3 = self.p4().next_table(page.p4_index());

        let huge_page = || {
            p3.and_then(|p3| {
                let p3_entry = &p3[page.p3_index()];
                // 1GiB page?
                if let Some(start_frame) = p3_entry.pointed_frame() {
                    if p3_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                        // address must be 1GiB aligned
                        assert_eq!(start_frame.start_address().get() % (ENTRY_COUNT * ENTRY_COUNT * PAGE_SIZE), 0);
                        return Some(Frame::of_addr(
                            start_frame.start_address().get() +
                                (page.p2_index() * ENTRY_COUNT + page.p1_index()) * PAGE_SIZE
                        ));
                    }
                }
                if let Some(p2) = p3.next_table(page.p3_index()) {
                    let p2_entry = &p2[page.p2_index()];
                    // 2MiB page?
                    if let Some(start_frame) = p2_entry.pointed_frame() {
                        if p2_entry.flags().contains(EntryFlags::HUGE_PAGE) {
                            // address must be 2MiB aligned
                            assert_eq!(start_frame.start_address().get() % ENTRY_COUNT, 0);
                            return Some(Frame::of_addr(
                                start_frame.start_address().get() + page.p1_index() * PAGE_SIZE
                            ));
                        }
                    }
                }
                None
            })
        };

        p3.and_then(|p3| p3.next_table(page.p3_index()))
            .and_then(|p2| p2.next_table(page.p2_index()))
            .and_then(|p1| p1[page.p1_index()].pointed_frame())
            .or_else(huge_page)
    }

    pub(super) fn entry_mut(&mut self, page: Page) -> &mut Entry {
        use core::ops::IndexMut;
        let p4 = self.p4_mut();
        let p3 = p4.next_table_create(page.p4_index());
        let p2 = p3.next_table_create(page.p3_index());
        let p1 = p2.next_table_create(page.p2_index());
        p1.index_mut(page.p1_index())
    }

    pub fn map_to(&mut self, page: Page, frame: Frame, flags: EntryFlags) {
        let entry = self.entry_mut(page);
        assert!(entry.is_unused());
        entry.set(frame, flags | EntryFlags::PRESENT);
    }

    pub fn map(&mut self, page: Page, flags: EntryFlags)
    {
        self.map_to(page, alloc_frame(), flags)
    }

    pub fn identity_map(&mut self, frame: Frame, flags: EntryFlags)
    {
        let page = Page::of_addr(frame.start_address().to_identity_virtual());
        self.map_to(page, frame, flags)
    }

    pub fn unmap(&mut self, page: Page) -> Frame
    {
        use x86_64::instructions::tlb;
        use x86_64::VirtAddr;

        assert!(self.translate(page.start_address()).is_some());

        let p1 = self.p4_mut()
            .next_table_mut(page.p4_index())
            .and_then(|p3| p3.next_table_mut(page.p3_index()))
            .and_then(|p2| p2.next_table_mut(page.p2_index()))
            .expect("mapping code does not support huge pages");
        let frame = p1[page.p1_index()].pointed_frame().unwrap();
        p1[page.p1_index()].set_unused();
        tlb::flush(VirtAddr::new(page.start_address() as u64));
        // TODO free p(1,2,3) table if empty
        frame
    }
}