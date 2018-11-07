#![feature(ptr_internals)]
#![feature(lang_items)]
#![feature(const_fn)]
#![feature(alloc)]
#![feature(naked_functions)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(panic_handler)]
#![feature(panic_info_message)]
#![feature(global_asm)]
#![feature(compiler_builtins_lib)]
#![feature(try_from)]
#![no_std]


#[macro_use]
extern crate alloc;
extern crate bit_allocator;
extern crate bit_field;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate linked_list_allocator;
#[macro_use]
extern crate log;
#[macro_use]
extern crate once;
extern crate simple_filesystem;
extern crate spin;
extern crate ucore_memory;
extern crate ucore_process;
extern crate volatile;
#[cfg(target_arch = "x86_64")]
extern crate x86_64;
extern crate xmas_elf;
extern crate usize_conversions;

use linked_list_allocator::LockedHeap;

#[macro_use]    // print!
pub mod logging;
mod memory;
mod lang;
mod util;
mod consts;
mod process;
mod syscall;
mod fs;

use process::{thread, thread_};
mod sync;
mod trap;
mod console;

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
    process::init();
    unsafe { arch::interrupt::enable(); }

    fs::shell();

//    thread::test::local_key();
//    thread::test::unpack();
//    sync::test::philosopher_using_mutex();
//    sync::test::philosopher_using_monitor();
//    sync::mpsc::test::test_all();

    loop {}
}

/// Global heap allocator
///
/// Available after `memory::init()`.
///
/// It should be defined in memory mod, but in Rust `global_allocator` must be in root mod.
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
