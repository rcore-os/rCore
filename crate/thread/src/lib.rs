#![cfg_attr(not(test), no_std)]
#![feature(alloc)]
#![feature(const_fn)]
#![feature(linkage)]
#![feature(vec_resize_default)]
#![feature(asm)]
#![feature(exact_size_is_empty)]

extern crate alloc;

mod interrupt;
mod processor;
pub mod scheduler;
pub mod std_thread;
mod thread_pool;
mod timer;

pub use crate::processor::Processor;
pub use crate::thread_pool::*;
