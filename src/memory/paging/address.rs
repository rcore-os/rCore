use consts::KERNEL_OFFSET;
pub use x86_64::{PhysicalAddress};
pub type VirtualAddress = usize;

pub trait FromToVirtualAddress {
	fn to_identity_virtual(&self) -> VirtualAddress;
	fn to_kernel_virtual(&self) -> VirtualAddress;
	fn from_kernel_virtual(addr: VirtualAddress) -> Self;
}

impl FromToVirtualAddress for PhysicalAddress {
	fn to_identity_virtual(&self) -> VirtualAddress {
		self.0 as usize
	}
	fn to_kernel_virtual(&self) -> VirtualAddress {
		self.0 as usize + KERNEL_OFFSET
	}
	fn from_kernel_virtual(addr: VirtualAddress) -> Self {
		PhysicalAddress((addr - KERNEL_OFFSET) as u64)
	}
}