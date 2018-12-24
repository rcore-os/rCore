// Physical address available on THINPAD:
// [0x80000000, 0x80800000]
#[cfg(target_arch = "riscv32")]
pub const RECURSIVE_INDEX: usize = 0x3fe;
#[cfg(target_arch = "riscv64")]
pub const RECURSIVE_INDEX: usize = 0x1fe;

pub const KERNEL_P2_INDEX: usize = 0x8000_0000 >> 22;
#[cfg(feature = "board_k210")]
pub const KERNEL_HEAP_SIZE: usize = 0x0010_0000;
#[cfg(not(feature = "board_k210"))]
pub const KERNEL_HEAP_SIZE: usize = 0x00a0_0000;
#[cfg(feature = "board_k210")]
pub const MEMORY_OFFSET: usize = 0x4000_0000;
#[cfg(not(feature = "board_k210"))]
pub const MEMORY_OFFSET: usize = 0x8000_0000;
#[cfg(feature = "board_k210")]
pub const MEMORY_END: usize = 0x4060_0000;
#[cfg(not(feature = "board_k210"))]
pub const MEMORY_END: usize = 0x8100_0000;

pub const USER_STACK_OFFSET: usize = 0x70000000;
pub const USER_STACK_SIZE: usize = 0x10000;
pub const USER32_STACK_OFFSET: usize = USER_STACK_OFFSET;
