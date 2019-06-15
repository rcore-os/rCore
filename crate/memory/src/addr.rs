use core::ops::{Add, AddAssign};

pub type VirtAddr = usize;
pub type PhysAddr = usize;

pub const PAGE_SIZE: usize = 1 << 12;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    /// Returns the start address of the page.
    pub fn start_address(&self) -> VirtAddr {
        self.number * PAGE_SIZE
    }

    /// Returns the page that contains the given virtual address.
    pub fn of_addr(addr: VirtAddr) -> Self {
        Page {
            number: addr / PAGE_SIZE,
        }
    }

    /// Returns a range of pages between address [begin, end)
    pub fn range_of(begin: VirtAddr, end: VirtAddr) -> PageRange {
        PageRange {
            start: Page::of_addr(begin),
            end: Page::of_addr(end - 1),
        }
    }
}

impl Add<usize> for Page {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        Page {
            number: self.number + rhs,
        }
    }
}

impl AddAssign<usize> for Page {
    fn add_assign(&mut self, rhs: usize) {
        *self = self.clone() + rhs;
    }
}

/// A range of pages with exclusive upper bound.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct PageRange {
    start: Page,
    end: Page,
}

impl Iterator for PageRange {
    type Item = Page;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let page = self.start.clone();
            self.start += 1;
            Some(page)
        } else {
            None
        }
    }
}
