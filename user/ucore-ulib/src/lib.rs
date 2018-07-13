#![no_std]
#![feature(asm)]
#![feature(lang_items)]
#![feature(panic_implementation)]
#![feature(panic_info_message)]
#![feature(linkage)]

#[macro_use]
pub mod syscall;
pub mod lang_items;