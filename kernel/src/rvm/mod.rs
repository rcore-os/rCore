//! Implement hypervisor using rvm crate

#![deny(non_upper_case_globals)]
#![deny(dead_code)]
#![deny(unused_mut)]
#![deny(unused_variables)]
#![deny(unused_imports)]

use rcore_fs::vfs::FsError;
use rvm::RvmError;

mod inode;
mod memory;
mod structs;

pub use inode::RvmINode;

fn into_fs_error(e: RvmError) -> FsError {
    match e {
        RvmError::Internal => FsError::DeviceError,
        RvmError::NotSupported => FsError::NotSupported,
        RvmError::NoMemory => FsError::NoDeviceSpace,
        RvmError::InvalidParam => FsError::InvalidParam,
        RvmError::OutOfRange => FsError::InvalidParam,
        RvmError::BadState => FsError::DeviceError,
        RvmError::NotFound => FsError::InvalidParam,
    }
}

mod rvm_extern_fn {
    use crate::memory::{alloc_frame_contiguous, dealloc_frame, phys_to_virt};
    use rvm::PAGE_SIZE;
    #[rvm::extern_fn(alloc_frames)]
    fn rvm_alloc_frames(n: usize, align_log2: usize) -> Option<usize> {
        alloc_frame_contiguous(n, align_log2)
    }

    #[rvm::extern_fn(dealloc_frames)]
    fn rvm_dealloc_frames(paddr: usize, n: usize, _align_log2: usize) {
        for i in 0..n {
            dealloc_frame(paddr + i * PAGE_SIZE)
        }
        //use crate::memory::dealloc_frame_contiguous;
        //dealloc_frame_contiguous(paddr, n, align_log2)
    }

    #[rvm::extern_fn(phys_to_virt)]
    fn rvm_phys_to_virt(paddr: usize) -> usize {
        phys_to_virt(paddr)
    }

    #[cfg(target_arch = "x86_64")]
    #[rvm::extern_fn(is_host_timer_interrupt)]
    fn rvm_x86_is_host_timer_interrupt(_vec: u8) -> bool {
        // TODO: fill in the blanks.
        false
    }
    #[cfg(target_arch = "x86_64")]
    #[rvm::extern_fn(is_host_serial_interrupt)]
    fn rvm_x86_is_host_serial_interrupt(_vec: u8) -> bool {
        // TODO: fill in the blanks.
        false
    }

    #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
    #[rvm::extern_fn(riscv_trap_handler_no_frame)]
    fn rvm_riscv_trap_handler_no_frame(sepc: &mut usize) {
        crate::arch::interrupt::trap_handler_no_frame(sepc);
    }

    #[cfg(any(target_arch = "riscv64", target_arch = "riscv32"))]
    #[rvm::extern_fn(riscv_check_hypervisor_extension)]
    fn rvm_riscv_check_hypervisor_extension() -> bool {
        if cfg!(feature = "hypervisor") {
            true
        } else {
            false
        }
    }
}
