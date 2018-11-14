#![no_std]
#![feature(alloc)]
#![feature(const_fn)]
#![feature(linkage)]
#![feature(nll)]
#![feature(vec_resize_default)]
#![feature(asm)]
#![feature(exact_size_is_empty)]

extern crate alloc;
#[macro_use]
extern crate log;
extern crate spin;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod process_manager;
mod processor;
pub mod scheduler;
pub mod thread;
mod event_hub;
mod interrupt;

pub use process_manager::*;
pub use processor::Processor;
