/// Platform specific constants
///
pub use super::board::consts::*;

pub const MEMORY_OFFSET: usize = 0x80000000;
pub const KERNEL_OFFSET: usize = 0x80100000;
pub const PHYSICAL_MEMORY_OFFSET: usize = 0x80000000;

pub const USER_STACK_OFFSET: usize = 0x70000000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 0x10000;

pub const MAX_DTB_SIZE: usize = 0x2000;
