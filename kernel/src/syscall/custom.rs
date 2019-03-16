//! Custom nonstandard syscalls
use rcore_memory::memory_set::handler::Linear;
use rcore_memory::memory_set::MemoryAttr;
use super::*;

#[cfg(target_arch = "x86_64")]
pub fn sys_map_pci_device(vendor: usize, product: usize) -> SysResult {
    use crate::drivers::bus::pci;
    info!(
        "map_pci_device: vendor: {}, product: {}",
        vendor, product
    );
    let tag = pci::find_device(vendor as u32, product as u32)
        .ok_or(SysError::ENOENT)?;
    // Get BAR0 memory
    let (base, len) = unsafe { tag.get_bar_mem(0) }
        .ok_or(SysError::ENOENT)?;

    let mut proc = process();
    let virt_addr = proc.memory_set.find_free_area(0, len);
    let attr = MemoryAttr::default().user();
    proc.memory_set.push(
        virt_addr,
        virt_addr + len,
        attr,
        Linear::new(base as isize - virt_addr as isize),
        "pci",
    );
    Ok(virt_addr)
}

#[cfg(not(target_arch = "x86_64"))]
pub fn sys_map_pci_device(vendor: usize, product: usize) -> SysResult {
    Err(SysError::ENOSYS)
}
