use consts::{KERNEL_OFFSET, KERNEL_SIZE};
pub use x86_64::{PhysicalAddress};
pub type VirtualAddress = usize;

pub trait FromToVirtualAddress {
	fn get(&self) -> usize;
	fn to_identity_virtual(&self) -> VirtualAddress;
	fn to_kernel_virtual(&self) -> VirtualAddress;
	fn from_kernel_virtual(addr: VirtualAddress) -> Self;
}

impl FromToVirtualAddress for PhysicalAddress {
	fn get(&self) -> usize {
		self.0 as usize
	}
	fn to_identity_virtual(&self) -> VirtualAddress {
		self.0 as usize
	}
	fn to_kernel_virtual(&self) -> VirtualAddress {
		assert!((self.0 as usize) < KERNEL_SIZE);
		self.0 as usize + KERNEL_OFFSET
	}
	fn from_kernel_virtual(addr: VirtualAddress) -> Self {
		assert!(addr >= KERNEL_OFFSET && addr < KERNEL_OFFSET + KERNEL_SIZE);
		PhysicalAddress((addr - KERNEL_OFFSET) as u64)
	}
}