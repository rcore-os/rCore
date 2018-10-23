#![no_std]
#![feature(alloc)]
#![feature(const_fn)]

extern crate alloc;
#[macro_use]
extern crate log;
extern crate spin;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

mod process_manager;
mod processor;
pub mod scheduler;
pub mod thread;
mod util;
mod event_hub;

pub use process_manager::*;
pub use processor::Processor;
