#![cfg_attr(not(test), no_std)]
#![feature(alloc)]
#![feature(const_fn)]
#![feature(linkage)]
#![feature(nll)]
#![feature(vec_resize_default)]
#![feature(asm)]
#![feature(exact_size_is_empty)]

extern crate alloc;

mod thread_pool;
mod processor;
pub mod scheduler;
pub mod std_thread;
mod timer;
mod interrupt;

pub use crate::thread_pool::*;
pub use crate::processor::Processor;
