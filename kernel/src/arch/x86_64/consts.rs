pub const MEMORY_OFFSET: usize = 0;
pub const KERNEL_OFFSET: usize = 0xffffff00_00000000;
pub const KERNEL_HEAP_SIZE: usize = 32 * 1024 * 1024; // 32 MB

pub const USER_STACK_OFFSET: usize = 0x00008000_00000000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 8 * 1024 * 1024; // 8 MB, the default config of Linux
