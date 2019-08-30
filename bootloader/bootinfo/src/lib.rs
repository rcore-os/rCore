//! Provides boot information to the kernel.
#![no_std]

#[cfg(target_arch = "aarch64")]
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub physical_memory_start: usize,
    pub physical_memory_end: usize,
    pub physical_memory_offset: usize,
}

#[cfg(target_arch = "mips")]
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    pub dtb: usize,
}
