#![no_std]
#![feature(asm)]

extern crate volatile;

mod asm;

pub mod gpio;
pub mod mini_uart;

pub const IO_BASE: usize = 0x3F000000;
