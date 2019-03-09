#[cfg(feature = "zero_kernel_offset")]
pub const KERNEL_OFFSET: usize = 0;
#[cfg(not(feature = "zero_kernel_offset"))]
pub const KERNEL_OFFSET: usize = 0xFFFF_0000_0000_0000;

pub const RAW_IO_BASE: usize = 0x3F00_0000;
pub const IO_BASE: usize = KERNEL_OFFSET + RAW_IO_BASE;
