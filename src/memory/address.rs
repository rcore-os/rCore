use consts::{KERNEL_OFFSET, KERNEL_SIZE};
pub use x86_64::PhysAddr;

pub type VirtAddr = usize;

pub trait FromToVirtualAddress {
	fn get(&self) -> usize;
	fn to_identity_virtual(&self) -> VirtAddr;
	fn to_kernel_virtual(&self) -> VirtAddr;
	fn from_kernel_virtual(addr: VirtAddr) -> Self;
}

impl FromToVirtualAddress for PhysAddr {
	fn get(&self) -> usize {
		self.as_u64() as usize
	}
	fn to_identity_virtual(&self) -> VirtAddr {
		self.get()
	}
	fn to_kernel_virtual(&self) -> VirtAddr {
		assert!(self.get() < KERNEL_SIZE);
		self.get() + KERNEL_OFFSET
	}
	fn from_kernel_virtual(addr: VirtAddr) -> Self {
		assert!(addr >= KERNEL_OFFSET && addr < KERNEL_OFFSET + KERNEL_SIZE);
		PhysAddr::new((addr - KERNEL_OFFSET) as u64)
	}
}