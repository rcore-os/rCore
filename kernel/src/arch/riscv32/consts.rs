// Physical address available on THINPAD:
// [0x80000000, 0x80800000]
#[cfg(target_arch = "riscv32")]
pub const RECURSIVE_INDEX: usize = 0x3fd;
#[cfg(target_arch = "riscv64")]
pub const RECURSIVE_INDEX: usize = 0o774;
// Under riscv64, upon booting, paging is enabled by bbl and
//  root_table[0777] maps to p3_table,
//   and p3_table[0777] maps to gigapage 8000_0000H,
//   so 0xFFFF_FFFF_8000_0000 maps to 0x8000_0000
//  root_table[0774] points to root_table itself as page table
//  root_table[0775] points to root_table itself as leaf page
//  root_table[0776] points to a temp page table as leaf page

#[cfg(target_arch = "riscv32")]
pub const KERNEL_OFFSET: usize = 0xC000_0000;
#[cfg(target_arch = "riscv64")]
pub const KERNEL_OFFSET: usize = 0xFFFF_FFFF_C000_0000;

#[cfg(target_arch = "riscv32")]
pub const KERNEL_P2_INDEX: usize = (KERNEL_OFFSET >> 12 >> 10) & 0x3ff;
#[cfg(target_arch = "riscv64")]
pub const KERNEL_P4_INDEX: usize = (KERNEL_OFFSET >> 12 >> 9 >> 9 >> 9) & 0o777;

pub const KERNEL_HEAP_SIZE: usize = 0x00a0_0000;

#[cfg(target_arch = "riscv32")]
pub const MEMORY_OFFSET: usize = 0x8000_0000;
#[cfg(target_arch = "riscv64")]
pub const MEMORY_OFFSET: usize = 0x8000_0000;

#[cfg(target_arch = "riscv32")]
pub const MEMORY_END: usize = 0x8100_0000;
#[cfg(target_arch = "riscv64")]
pub const MEMORY_END: usize = 0x8100_0000;

// FIXME: rv64 `sh` and `ls` will crash if stack top > 0x80000000 ???
pub const USER_STACK_OFFSET: usize = 0x80000000 - USER_STACK_SIZE;
pub const USER_STACK_SIZE: usize = 0x10000;
pub const USER32_STACK_OFFSET: usize = 0xC0000000 - USER_STACK_SIZE;

pub const MAX_DTB_SIZE: usize = 0x2000;