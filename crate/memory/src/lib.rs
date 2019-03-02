#![cfg_attr(not(test), no_std)]
#![feature(alloc)]
#![feature(nll)]

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

pub enum VMError {
    InvalidPtr
}

pub type VMResult<T> = Result<T, VMError>;