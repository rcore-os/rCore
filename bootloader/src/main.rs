#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(global_asm)]

#[macro_use]
extern crate fixedvec;
extern crate xmas_elf;

use core::mem::transmute;
use core::slice;
use fixedvec::FixedVec;
use xmas_elf::{
    header,
    program::{ProgramHeader, ProgramHeader32, ProgramHeader64},
    ElfFile,
};

#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64/mod.rs"]
pub mod arch;

#[cfg(target_arch = "mips")]
#[path = "arch/mipsel/mod.rs"]
pub mod arch;

pub mod lang_items;

extern "C" {
    fn _kernel_payload_start();
    fn _kernel_payload_end();
}

#[cfg(target_arch = "mips")]
extern "C" {
    fn _dtb_start();
    fn _dtb_end();
}

/// The entry point of bootloader
#[cfg(target_arch = "aarch64")]
#[no_mangle]
pub extern "C" fn boot_main() -> ! {
    let kernel_size = _kernel_payload_end as usize - _kernel_payload_start as usize;
    let kernel = unsafe { slice::from_raw_parts(_kernel_payload_start as *const u8, kernel_size) };
    let kernel_elf = ElfFile::new(kernel).unwrap();
    header::sanity_check(&kernel_elf).unwrap();

    let mut preallocated_space = alloc_stack!([ProgramHeader64; 32]);
    let mut segments = FixedVec::new(&mut preallocated_space);

    for program_header in kernel_elf.program_iter() {
        match program_header {
            ProgramHeader::Ph64(header) => segments
                .push(*header)
                .expect("does not support more than 32 program segments"),
            ProgramHeader::Ph32(_) => panic!("does not support 32 bit elf files"),
        }
    }

    let entry = kernel_elf.header.pt2.entry_point();
    let kernel_main: extern "C" fn() = unsafe { transmute(entry) };

    arch::map_kernel(_kernel_payload_start as usize, &segments);
    kernel_main();

    loop {}
}


#[cfg(target_arch = "mips")]
#[no_mangle]
pub extern "C" fn boot_main() -> ! {
    let kernel_size = _kernel_payload_end as usize - _kernel_payload_start as usize;
    let kernel = unsafe { slice::from_raw_parts(_kernel_payload_start as *const u8, kernel_size) };
    let kernel_elf = ElfFile::new(kernel).unwrap();
    header::sanity_check(&kernel_elf).unwrap();

    let mut preallocated_space = alloc_stack!([ProgramHeader32; 32]);
    let mut segments = FixedVec::new(&mut preallocated_space);

    for program_header in kernel_elf.program_iter() {
        match program_header {
            ProgramHeader::Ph32(header) => segments
                .push(*header)
                .expect("does not support more than 32 program segments"),
            ProgramHeader::Ph64(_) => panic!("does not support 64 bit elf files"),
        }
    }

    let entry = kernel_elf.header.pt2.entry_point() as u32;
    let kernel_main: extern "C" fn(dtb: usize) = unsafe { transmute(entry) };

    arch::copy_kernel(_kernel_payload_start as usize, &segments);
    kernel_main(_dtb_start as usize);

    loop {}
}
