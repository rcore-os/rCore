#![no_std]
#![feature(alloc)]

extern crate alloc;

pub mod physical;
pub mod paging;
pub mod memory_set;
pub mod swap;

type VirtAddr = usize;
const PAGE_SIZE: usize = 4096;