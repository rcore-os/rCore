// Physical address available on THINPAD:
// [0x80000000, 0x80800000]
const P2_SIZE: usize = 1 << 22;
const P2_MASK: usize = 0x3ff << 22;
pub const RECURSIVE_PAGE_PML4: usize = 0x3fe;
pub const KERNEL_OFFSET: usize = 0;
pub const KERNEL_PML4: usize = 0x8000_0000 >> 22;
pub const KERNEL_HEAP_OFFSET: usize = 0x8020_0000;
pub const KERNEL_HEAP_SIZE: usize = 0x0020_0000;
pub const MEMORY_OFFSET: usize = 0x8000_0000;
pub const MEMORY_END: usize = 0x8080_0000;
pub const USER_STACK_OFFSET: usize = 0x70000000;
pub const USER_STACK_SIZE: usize = 0x10000;
pub const USER32_STACK_OFFSET: usize = USER_STACK_OFFSET;