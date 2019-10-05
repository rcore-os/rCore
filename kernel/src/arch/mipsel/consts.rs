/// Platform specific constants
///
pub use super::board::consts::*;

pub const MEMORY_OFFSET: usize = 0x8000_0000;

#[cfg(feature = "board_thinpad")]
pub const KERNEL_OFFSET: usize = 0x8000_0000;
#[cfg(not(feature = "board_thinpad"))]
pub const KERNEL_OFFSET: usize = 0x8010_0000;

pub const PHYSICAL_MEMORY_OFFSET: usize = 0x8000_0000;

pub const USER_STACK_OFFSET: usize = 0x7000_0000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 0x10000;

pub const MAX_DTB_SIZE: usize = 0x2000;

pub const KSEG2_START: usize = 0xfe80_0000;
