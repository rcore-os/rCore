#![no_std]
#![feature(alloc)]
#![feature(universal_impl_trait)]
#![feature(match_default_bindings)]

extern crate alloc;

// To use `println!` in test
#[cfg(test)]
#[macro_use]
extern crate std;

pub mod paging;
// FIXME: LLVM error on riscv32
#[cfg(target_arch = "x86_64")]
pub mod cow;
pub mod swap;

type VirtAddr = usize;
type PhysAddr = usize;
const PAGE_SIZE: usize = 4096;