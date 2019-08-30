#![no_std]
#![no_main]
#![feature(lang_items)]
#![feature(global_asm)]

#[macro_use]
extern crate fixedvec;
extern crate xmas_elf;

use bootinfo::BootInfo;
use core::mem::transmute;
use core::slice;
use fixedvec::FixedVec;
use xmas_elf::{
    header,
    program::{self, ProgramHeader},
    ElfFile,
};

#[cfg(target_arch = "aarch64")]
#[path = "arch/aarch64/mod.rs"]
mod arch;

#[cfg(target_arch = "mips")]
#[path = "arch/mipsel/mod.rs"]
mod arch;

mod lang_items;

extern "C" {
    fn _kernel_payload_start();
    fn _kernel_payload_end();
}

global_asm!(concat!(
    r#"
    .section .payload,"a"
    .align 12
    .global _kernel_payload_start, _kernel_payload_end
_kernel_payload_start:
    .incbin ""#,
    env!("PAYLOAD"),
    r#""
_kernel_payload_end:
"#
));

/// The entry point of bootloader
#[no_mangle]
pub extern "C" fn boot_main() -> ! {
    let kernel_size = _kernel_payload_end as usize - _kernel_payload_start as usize;
    let kernel = unsafe { slice::from_raw_parts(_kernel_payload_start as *const u8, kernel_size) };
    let kernel_elf = ElfFile::new(kernel).unwrap();
    header::sanity_check(&kernel_elf).unwrap();

    #[cfg(target_pointer_width = "64")]
    let mut preallocated_space = alloc_stack!([program::ProgramHeader64; 32]);
    #[cfg(target_pointer_width = "32")]
    let mut preallocated_space = alloc_stack!([program::ProgramHeader32; 32]);
    let mut segments = FixedVec::new(&mut preallocated_space);

    for program_header in kernel_elf.program_iter() {
        if let Some(header) = match program_header {
            #[cfg(target_pointer_width = "64")]
            ProgramHeader::Ph64(h) => Some(h),
            #[cfg(target_pointer_width = "32")]
            ProgramHeader::Ph32(h) => Some(h),
            _ => None,
        } {
            segments
                .push(*header)
                .expect("does not support more than 32 program segments")
        } else {
            panic!("does not support 32 bit elf files")
        }
    }

    let entry = kernel_elf.header.pt2.entry_point() as usize;
    let kernel_main: extern "C" fn(boot_info_addr: usize) = unsafe { transmute(entry) };

    let (boot_info, boot_info_addr) = arch::copy_kernel(_kernel_payload_start as usize, &segments);
    unsafe { (boot_info_addr as *mut BootInfo).write(boot_info) };
    kernel_main(boot_info_addr);

    loop {}
}
