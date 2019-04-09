#[cfg(any(target_arch = "x86_64", target_arch = "mips"))]
pub mod pci;
pub mod virtio_mmio;
