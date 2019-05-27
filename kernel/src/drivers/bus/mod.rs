#[cfg(any(
    target_arch = "x86_64",
    all(target_arch = "mips", feature = "board_malta")
))]
pub mod pci;
pub mod virtio_mmio;
