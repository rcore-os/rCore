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
    use crate::memory::{alloc_frame, dealloc_frame, phys_to_virt};
    #[rvm::extern_fn(alloc_frame)]
    fn rvm_alloc_frame() -> Option<usize> {
        alloc_frame()
    }

    #[rvm::extern_fn(dealloc_frame)]
    fn rvm_dealloc_frame(paddr: usize) {
        dealloc_frame(paddr)
    }
    #[rvm::extern_fn(alloc_frame_x4)]
    fn rvm_alloc_frame_x4() -> Option<usize> {
        use crate::memory::alloc_frame_contiguous;
        alloc_frame_contiguous(4, 2)
    }

    #[rvm::extern_fn(dealloc_frame_x4)]
    fn rvm_dealloc_frame_x4(paddr: usize) {
        dealloc_frame(paddr)
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

    #[cfg(all(
        any(target_arch = "riscv64", target_arch = "riscv32"),
        feature = "hypervisor"
    ))]
    #[rvm::extern_fn(riscv_check_hypervisor_extension)]
    fn rvm_riscv_check_hypervisor_extension() -> bool {
        return true;
    }
    #[cfg(all(
        any(target_arch = "riscv64", target_arch = "riscv32"),
        not(feature = "hypervisor")
    ))]
    #[rvm::extern_fn(riscv_check_hypervisor_extension)]
    fn rvm_riscv_check_hypervisor_extension() -> bool {
        return false;
    }
}
