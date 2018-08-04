#![no_std]
#![feature(alloc)]
#![feature(const_fn)]

extern crate alloc;
#[macro_use]
extern crate log;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod processor;
pub mod scheduler;
pub mod thread;
mod util;
mod event_hub;
