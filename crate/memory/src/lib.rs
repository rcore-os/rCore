#![no_std]
#![feature(alloc)]
#![feature(universal_impl_trait, conservative_impl_trait)]
#![feature(match_default_bindings)]

extern crate alloc;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod paging;
pub mod cow;
pub mod swap;
pub mod memory_set;
mod addr;

pub use addr::*;