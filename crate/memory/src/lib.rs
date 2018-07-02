#![no_std]
#![feature(alloc)]

extern crate alloc;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod paging;
pub mod cow;
pub mod swap;

type VirtAddr = usize;
type PhysAddr = usize;
const PAGE_SIZE: usize = 4096;