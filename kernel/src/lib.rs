#![feature(lang_items)]
#![feature(alloc)]
#![feature(naked_functions)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(panic_info_message)]
#![feature(global_asm)]
#![feature(extern_crate_item_prelude)]
#![no_std]

// just keep it ...
extern crate alloc;

pub use crate::process::{processor, new_kernel_context};
use ucore_process::thread;
use linked_list_allocator::LockedHeap;

#[macro_use]    // print!
mod logging;
mod memory;
mod lang;
mod util;
mod consts;
mod process;
mod syscall;
mod fs;
mod sync;
mod trap;
mod shell;

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
pub mod arch;

#[cfg(target_arch = "riscv32")]
#[path = "arch/riscv32/mod.rs"]
pub mod arch;

#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64/mod.rs"]
pub mod arch;

pub fn kmain() -> ! {
    processor().run();
}

/// Global heap allocator
///
/// Available after `memory::init()`.
///
/// It should be defined in memory mod, but in Rust `global_allocator` must be in root mod.
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
