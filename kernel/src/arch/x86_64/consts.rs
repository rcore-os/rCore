pub const MEMORY_OFFSET: usize = 0;
pub const KERNEL_OFFSET: usize = 0xffffff00_00000000;
pub const KSEG2_OFFSET: usize = 0xfffffe80_00000000;
pub const PHYSICAL_MEMORY_OFFSET: usize = 0xffff8000_00000000;
pub const KERNEL_HEAP_SIZE: usize = 8 * 1024 * 1024; // 8 MB

pub const KERNEL_PM4: usize = (KERNEL_OFFSET >> 39) & 0o777;
pub const KSEG2_PM4: usize = (KSEG2_OFFSET >> 39) & 0o777;
pub const PHYSICAL_MEMORY_PM4: usize = (PHYSICAL_MEMORY_OFFSET >> 39) & 0o777;

pub const USER_STACK_OFFSET: usize = 0x00008000_00000000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB, the default config of Linux
pub const KSEG2_START: usize = 0xffff_fe80_0000_0000;
