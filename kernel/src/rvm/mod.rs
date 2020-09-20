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

    #[rvm::extern_fn(phys_to_virt)]
    fn rvm_phys_to_virt(paddr: usize) -> usize {
        phys_to_virt(paddr)
    }

    #[cfg(target_arch = "x86_64")]
    #[rvm::extern_fn(x86_all_traps_handler_addr)]
    unsafe fn rvm_x86_all_traps_handler_addr() -> usize {
        extern "C" {
            fn __alltraps();
        }
        __alltraps as usize
    }
}
