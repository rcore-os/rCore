#![cfg_attr(not(test), no_std)]
#![feature(alloc)]
#![feature(nll)]
#![feature(extern_crate_item_prelude)]

// import macros from log
use log::*;
extern crate alloc;

pub mod paging;
pub mod cow;
pub mod swap;
pub mod memory_set;
mod addr;
pub mod no_mmu;

pub use crate::addr::*;