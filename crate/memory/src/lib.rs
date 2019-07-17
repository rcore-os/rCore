#![cfg_attr(not(test), no_std)]
#![feature(nll)]

// import macros from log
use log::*;
extern crate alloc;

mod addr;
pub mod cow;
pub mod memory_set;
pub mod no_mmu;
pub mod paging;
//pub mod swap;

pub use crate::addr::*;

pub enum VMError {
    InvalidPtr,
}

pub type VMResult<T> = Result<T, VMError>;
