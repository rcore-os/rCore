#![cfg_attr(not(test), no_std)]
#![feature(alloc)]
#![feature(const_fn)]
#![feature(linkage)]
#![feature(nll)]
#![feature(vec_resize_default)]
#![feature(asm)]
#![feature(exact_size_is_empty)]

extern crate alloc;

mod process_manager;
mod processor;
pub mod scheduler;
pub mod thread;
mod event_hub;
mod interrupt;

pub use crate::process_manager::*;
pub use crate::processor::Processor;
