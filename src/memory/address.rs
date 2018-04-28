use consts::{KERNEL_OFFSET, KERNEL_SIZE};
pub use x86_64::PhysicalAddress as PhysAddr;
pub type VirtAddr = usize;

pub trait FromToVirtualAddress {
	fn get(&self) -> usize;
	fn to_identity_virtual(&self) -> VirtAddr;
	fn to_kernel_virtual(&self) -> VirtAddr;
	fn from_kernel_virtual(addr: VirtAddr) -> Self;
}

impl FromToVirtualAddress for PhysAddr {
	fn get(&self) -> usize {
		self.0 as usize
	}
	fn to_identity_virtual(&self) -> VirtAddr {
		self.0 as usize
	}
	fn to_kernel_virtual(&self) -> VirtAddr {
		assert!((self.0 as usize) < KERNEL_SIZE);
		self.0 as usize + KERNEL_OFFSET
	}
	fn from_kernel_virtual(addr: VirtAddr) -> Self {
		assert!(addr >= KERNEL_OFFSET && addr < KERNEL_OFFSET + KERNEL_SIZE);
		PhysAddr((addr - KERNEL_OFFSET) as u64)
	}
}