#![feature(lang_items)]
#![feature(naked_functions)]
#![feature(untagged_unions)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(panic_info_message)]
#![feature(global_asm)]
#![feature(alloc_prelude)]
#![deny(unused_must_use)]
#![deny(stable_features)]
#![deny(unused_unsafe)]
#![deny(ellipsis_inclusive_range_patterns)]
#![no_std]

// just keep it ...
#[macro_use]
extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

pub use crate::process::{new_kernel_context, processor};
pub use buddy_system_allocator::LockedHeapWithRescue;
pub use rcore_thread::std_thread as thread;

#[macro_use] // print!
pub mod logging;
#[macro_use]
pub mod util;
pub mod backtrace;
pub mod consts;
pub mod drivers;
pub mod fs;
pub mod lang;
pub mod lkm;
pub mod memory;
pub mod net;
pub mod process;
pub mod shell;
pub mod sync;
pub mod syscall;
pub mod trap;

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
pub mod arch;

#[cfg(target_arch = "mips")]
#[path = "arch/mipsel/mod.rs"]
pub mod arch;

#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
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
static HEAP_ALLOCATOR: LockedHeapWithRescue =
    LockedHeapWithRescue::new(crate::memory::enlarge_heap);
