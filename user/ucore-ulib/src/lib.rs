#![no_std]
#![feature(asm)]
#![feature(lang_items)]
#![feature(panic_info_message)]
#![feature(linkage)]
#![feature(compiler_builtins_lib)]

#[macro_use]
pub mod syscall;
pub mod lang_items;