pub use self::entry::*;
pub use self::mapper::Mapper;
use core::ops::{Deref, DerefMut, Add};
use memory::*;
pub use self::temporary_page::TemporaryPage;
pub use self::cow::*;

mod entry;
mod table;
mod temporary_page;
mod mapper;
mod cow;

const ENTRY_COUNT: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
   number: usize,
}

impl Page {
    pub fn of_addr(address: VirtAddr) -> Page {
        assert!(address < 0x0000_8000_0000_0000 ||
            address >= 0xffff_8000_0000_0000,
            "invalid address: 0x{:x}", address);
        Page { number: address / PAGE_SIZE }
    }

    pub fn start_address(&self) -> usize {
        self.number * PAGE_SIZE
    }

    fn p4_index(&self) -> usize {
        (self.number >> 27) & 0o777
    }
    fn p3_index(&self) -> usize {
        (self.number >> 18) & 0o777
    }
    fn p2_index(&self) -> usize {
        (self.number >> 9) & 0o777
    }
    fn p1_index(&self) -> usize {
        (self.number >> 0) & 0o777
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start,
            end,
        }
    }

    /// Iterate pages of address [begin, end)
    pub fn range_of(begin: VirtAddr, end: VirtAddr) -> PageIter {
        PageIter {
            start: Page::of_addr(begin),
            end: Page::of_addr(end - 1),
        }
    }
}

impl Add<usize> for Page {
    type Output = Page;

    fn add(self, rhs: usize) -> Page {
        Page { number: self.number + rhs }
    }
}


#[derive(Clone)]
pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        } else {
            None
        }
    }
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    pub const unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub fn with(&mut self, table: &mut InactivePageTable, f: impl FnOnce(&mut Mapper))
    {
        use x86_64::instructions::tlb;
        use x86_64::registers::control_regs;

        let mut temporary_page = TemporaryPage::new(Page::of_addr(0xcafebabe));
        {
            let backup = Frame::of_addr(
                control_regs::cr3().0 as usize);

            // map temporary_page to current p4 table
            let p4_table = temporary_page.map_table_frame(backup.clone(), self);

            // overwrite recursive mapping
            self.p4_mut()[511].set(table.p4_frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();

            // execute f in the new context
            f(self);

            // restore recursive mapping to original p4 table
            p4_table[511].set(backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }

        temporary_page.unmap(self);
    }

    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use x86_64::PhysicalAddress;
        use x86_64::registers::control_regs;
        debug!("switch table to {:?}", new_table.p4_frame);

        let old_table = InactivePageTable {
            p4_frame: Frame::of_addr(control_regs::cr3().0 as usize),
        };
        unsafe {
            control_regs::cr3_write(new_table.p4_frame.start_address());
        }
        old_table
    }
}

#[derive(Debug)]
pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(frame: Frame, active_table: &mut ActivePageTable) -> InactivePageTable {
        let mut temporary_page = TemporaryPage::new(Page::of_addr(0xcafebabe));
        {
            let table = temporary_page.map_table_frame(frame.clone(),
                active_table);
            // now we are able to zero the table
            table.zero();
            // set up recursive mapping for the table
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE);
        }
        temporary_page.unmap(active_table);

        InactivePageTable { p4_frame: frame }
    }
}