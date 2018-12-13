pub const RECURSIVE_INDEX: usize = 0o777;
pub const KERNEL_OFFSET: usize = 0;
pub const KERNEL_PML4: usize = 0;
pub const KERNEL_HEAP_SIZE: usize = 8 * 1024 * 1024;
pub const MEMORY_OFFSET: usize = 0;
pub const USER_STACK_OFFSET: usize = 0xffff_8000_0000_0000;
pub const USER_STACK_SIZE: usize = 1 * 1024 * 1024;
pub const USER32_STACK_OFFSET: usize = USER_STACK_OFFSET;