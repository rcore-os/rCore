use core::ops::{Add, AddAssign};
use super::paging::*;
use super::memory_set::*;

pub type VirtAddr = usize;
pub type PhysAddr = usize;
pub const PAGE_SIZE: usize = 1 << 12;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: usize,
}

impl Page {
    /*
    **  @brief  get the virtual address of beginning of the page
    **  @retval VirtAddr             the virtual address of beginning of the page
    */
    pub fn start_address(&self) -> VirtAddr {
        self.number * PAGE_SIZE
    }
    /*
    **  @brief  get the page of a given virtual address
    **  @param  addr: VirtAddr       the given virtual address
    **  @retval Page                 the page of the given virtual address
    */
    pub fn of_addr(addr: VirtAddr) -> Self {
        Page { number: addr / PAGE_SIZE }
    }

    /*
    **  @brief  get a pageRange between two virtual address
    **  @param  begin: VirtAddr      the virtual address of the beginning
    **  @param  end: VirtAddr        the virtual address of the end
    **  @retval PageRange            the page of the given virtual address
    */
    pub fn range_of(begin: VirtAddr, end: VirtAddr) -> PageRange {
        PageRange {
            start: Page::of_addr(begin),
            end: Page::of_addr(end - 1) + 1,
        }
    }
}

impl Add<usize> for Page {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        Page { number: self.number + rhs }
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
        if self.start < self.end {
            let page = self.start.clone();
            self.start += 1;
            Some(page)
        } else {
            None
        }
    }
}

/// frame for the swapmanager
#[derive(Debug, Copy, Clone, PartialOrd, Ord)]
#[repr(C)]
pub struct Frame  {
    page_table: usize, // the raw pointer for the frame's memory set's inactive page table
    virtaddr: VirtAddr, // the virtual addr for the frame
    token: usize, // the token for frame
}

impl Frame{
    pub fn get_page_table(&self) -> usize {
        self.page_table
    }

    pub fn get_virtaddr(&self) -> VirtAddr {
        self.virtaddr
    }

    pub fn get_token(&self) -> usize{
        self.token
    }

    pub fn new(pt: usize, addr: VirtAddr, pttoken: usize) -> Self{
        Frame {
            page_table: pt,
            virtaddr: addr,
            token: pttoken,
        }
    }
} 

impl PartialEq for Frame{
    fn eq(&self, other: &Frame) -> bool {
        self.token == other.token && self.virtaddr == other.virtaddr
    }
}

impl Eq for Frame{
    
}