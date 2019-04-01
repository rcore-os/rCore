// Physical address available on THINPAD:
// [0x80000000, 0x80800000]

pub const KERNEL_OFFSET: usize = 0xC000_0000;

#[cfg(target_arch = "riscv32")]
pub const KERNEL_P2_INDEX: usize = (KERNEL_OFFSET >> 12 >> 10) & 0x3ff;

pub const KERNEL_HEAP_SIZE: usize = 0x00a0_0000;

pub const MEMORY_OFFSET: usize = 0x8000_0000;
pub const MEMORY_END: usize = 0x8100_0000;

pub const USER_STACK_OFFSET: usize = 0x80000000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 0x10000;
pub const USER32_STACK_OFFSET: usize = 0xC0000000 - USER_STACK_SIZE;

pub const MAX_DTB_SIZE: usize = 0x2000;
