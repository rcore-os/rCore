pub const MEMORY_OFFSET: usize = 0;
pub const KERNEL_OFFSET: usize = 0xFFFF_0000_0000_0000;
pub const PHYSICAL_MEMORY_OFFSET: usize = 0xFFFF_8000_0000_0000;
pub const KERNEL_HEAP_SIZE: usize = 8 * 1024 * 1024;

pub const USER_STACK_OFFSET: usize = 0x0000_8000_0000_0000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 1 * 1024 * 1024;
pub const KSEG2_START: usize = 0xffff_fe80_0000_0000;
