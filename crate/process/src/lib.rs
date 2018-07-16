#![no_std]
#![feature(alloc)]
#![feature(const_fn)]
#![feature(linkage)]
#![feature(universal_impl_trait, conservative_impl_trait)]

extern crate alloc;
#[macro_use]
extern crate log;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod processor;
pub mod scheduler;
mod util;
mod event_hub;
