//! Custom nonstandard syscalls
use super::*;
use rcore_memory::memory_set::handler::Linear;
use rcore_memory::memory_set::MemoryAttr;

impl Syscall<'_> {
    /// Allocate this PCI device to user space
    /// The kernel driver using the PCI device will be unloaded
    #[cfg(target_arch = "x86_64")]
    pub fn sys_map_pci_device(&mut self, vendor: usize, product: usize) -> SysResult {
        use crate::drivers::bus::pci;
        info!(
            "map_pci_device: vendor: {:x}, product: {:x}",
            vendor, product
        );

        let tag = pci::find_device(vendor as u16, product as u16).ok_or(SysError::ENOENT)?;
        if pci::detach_driver(&tag) {
            info!("Kernel driver detached");
        }

        // Get BAR0 memory
        let (base, len) = pci::get_bar0_mem(tag).ok_or(SysError::ENOENT)?;

        let virt_addr = self.vm().find_free_area(0, len);
        let attr = MemoryAttr::default().user();
        self.vm().push(
            virt_addr,
            virt_addr + len,
            attr,
            Linear::new(base as isize - virt_addr as isize),
            "pci",
        );
        Ok(virt_addr)
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn sys_map_pci_device(&mut self, vendor: usize, product: usize) -> SysResult {
        Err(SysError::ENOSYS)
    }

    /// Get start physical addresses of frames
    /// mapped to a list of virtual addresses.
    pub fn sys_get_paddr(
        &mut self,
        vaddrs: *const u64,
        paddrs: *mut u64,
        count: usize,
    ) -> SysResult {
        let vaddrs = unsafe { self.vm().check_read_array(vaddrs, count)? };
        let paddrs = unsafe { self.vm().check_write_array(paddrs, count)? };
        for i in 0..count {
            let paddr = self.vm().translate(vaddrs[i] as usize).unwrap_or(0);
            paddrs[i] = paddr as u64;
        }
        Ok(0)
    }
}
