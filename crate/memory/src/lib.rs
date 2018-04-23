#![no_std]
#![feature(alloc)]

extern crate alloc;

pub mod memory_set;
pub mod swap;
pub mod page_table;

type VirtAddr = usize;