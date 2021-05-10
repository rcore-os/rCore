//! Wrappers of rvm::Guest and rvm::Vcpu

use alloc::sync::Arc;
use spin::Mutex;

use rcore_memory::{memory_set::MemoryAttr, PAGE_SIZE};
use rvm::{DefaultGuestPhysMemorySet, GuestPhysAddr, HostVirtAddr, RvmResult};
use rvm::{Guest as GuestInner, Vcpu as VcpuInner};

use super::memory::RvmPageTableHandlerDelay;
use crate::memory::GlobalFrameAlloc;

pub(super) struct Guest {
    gpm: Arc<DefaultGuestPhysMemorySet>,
    pub(super) inner: Arc<GuestInner>,
}

pub(super) struct Vcpu {
    pub(super) inner: Mutex<VcpuInner>,
    //#[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    //inner_id: usize,
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    pub(super) irq: Arc<rvm::InterruptState>,
}

impl Guest {
    pub fn new() -> RvmResult<Self> {
        let gpm = DefaultGuestPhysMemorySet::new();
        Ok(Self {
            inner: GuestInner::new(gpm.clone())?,
            gpm,
        })
    }

    pub fn add_memory_region(&self, gpaddr: GuestPhysAddr, size: usize) -> RvmResult<HostVirtAddr> {
        self.inner.add_memory_region(gpaddr, size, None)?;
        let thread = crate::process::current_thread().unwrap();
        let hvaddr = thread.vm.lock().find_free_area(PAGE_SIZE, size);
        let handler =
            RvmPageTableHandlerDelay::new(gpaddr, hvaddr, self.gpm.clone(), GlobalFrameAlloc);
        thread.vm.lock().push(
            hvaddr,
            hvaddr + size,
            MemoryAttr::default().user().writable(),
            handler,
            "rvm_guest_physical",
        );
        Ok(hvaddr)
    }
}

impl Vcpu {
    pub fn new(entry: u64, guest: Arc<GuestInner>) -> RvmResult<Self> {
        #[cfg(any(target_arch = "x86_64"))]
        {
            Ok(Self {
                inner: Mutex::new(VcpuInner::new(entry, guest)?),
            })
        }
        #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
        {
            let inner = VcpuInner::new(entry, Arc::clone(&guest))?;
            let inner_id = inner.get_id();
            let irq = guest.get_irq_by_id(inner_id);
            return Ok(Self {
                inner: Mutex::new(inner),
                //inner_id,
                irq,
            });
        }
    }
}
