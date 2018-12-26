// Physical address available on THINPAD:
// [0x80000000, 0x80800000]
#[cfg(target_arch = "riscv32")]
pub const RECURSIVE_INDEX: usize = 0x3fe;
#[cfg(target_arch = "riscv64")]
pub const RECURSIVE_INDEX: usize = 0x1fd;
// Under riscv64, upon booting, paging is enabled by bbl and
//  root_table[0777] maps to p3_table,
//   and p3_table[0776] maps to gigapage 8000_0000H,
//   so 0xFFFF_FFFF_8000_0000 maps to 0x8000_0000
//  root_table[0775] points to root_table itself as page table
//  root_table[0776] points to root_table itself as leaf page

#[cfg(target_arch = "riscv32")]
pub const KERN_VA_BASE: usize = 0;
#[cfg(target_arch = "riscv64")]
pub const KERN_VA_BASE: usize = 0xFFFF_FFFF_0000_0000;

#[cfg(target_arch = "riscv32")]
pub const KERNEL_P2_INDEX: usize = 0x8000_0000 >> 12 >> 10;
#[cfg(target_arch = "riscv64")]
pub const KERNEL_P4_INDEX: usize = 0x0000_FFFF_8000_0000 >> 12 >> 9 >> 9 >> 9;

#[cfg(feature = "board_k210")]
pub const KERNEL_HEAP_SIZE: usize = 0x0010_0000;
#[cfg(not(feature = "board_k210"))]
pub const KERNEL_HEAP_SIZE: usize = 0x00a0_0000;

#[cfg(feature = "board_k210")]
pub const MEMORY_OFFSET: usize = 0x4000_0000;
#[cfg(target_arch = "riscv32")]
pub const MEMORY_OFFSET: usize = 0x8000_0000;
#[cfg(all(target_arch = "riscv64", not(feature = "board_k210")))]
pub const MEMORY_OFFSET: usize = 0x8000_0000;

#[cfg(target_arch = "riscv32")]
pub const MEMORY_END: usize = 0x8100_0000;
#[cfg(feature = "board_k210")]
pub const MEMORY_END: usize = 0x4060_0000;
#[cfg(all(target_arch = "riscv64", not(feature = "board_k210")))]
pub const MEMORY_END: usize = 0x8100_0000;

pub const USER_STACK_OFFSET: usize = 0x70000000;
pub const USER_STACK_SIZE: usize = 0x10000;
pub const USER32_STACK_OFFSET: usize = USER_STACK_OFFSET;
