// Migrate from xv6-x86_64 acpi.c

mod structs;
use self::structs::*;

/// See https://wiki.osdev.org/RSDP -- Detecting the RSDP
pub fn find_rsdp() -> Option<&'static rsdp> {
	use util::{Checkable, find_in_memory};
	let ebda = unsafe { *(0x40E as *const u16) as usize } << 4;
	debug!("EBDA at {:#x}", ebda);

	macro_rules! return_if_find_in {
		($begin:expr, $end:expr) => (
			if let Some(addr) = unsafe{ find_in_memory::<rsdp>($begin, $end, 4) } {
				return Some(unsafe{ &*(addr as *const rsdp) });
			}
		)
	}
	
	if ebda != 0 {
		return_if_find_in!(ebda as usize, 1024);
	}
	return_if_find_in!(0xE0000, 0x20000);
	None
} 

#[cfg(target_arch="x86")]
const PHYSICAL_MEMORY_LIMIT: u32 = 0x0E000000;
#[cfg(target_arch="x86_64")]
const PHYSICAL_MEMORY_LIMIT: u32 = 0x80000000;

#[derive(Debug)]
pub enum ACPI_Error {
	NotMapped
}

pub fn init() -> Result<(), ACPI_Error> {
	use core::mem::size_of;
	use util::Checkable;
	let rsdp = find_rsdp().expect("acpi: rsdp not found.");
	if rsdp.RsdtPhysicalAddress > PHYSICAL_MEMORY_LIMIT {
		return Err(ACPI_Error::NotMapped);
	}
	let rsdt = unsafe{ &*(rsdp.RsdtPhysicalAddress as *const rsdt) };
	let entry_count = (rsdt.Header.Length as usize - size_of::<header>()) / 4;
	let mut madt: Option<&'static madt> = None;
	for i in 0 ..  entry_count {
		let entry = unsafe{ rsdt.entry_at(i) };
		if entry > PHYSICAL_MEMORY_LIMIT {
			return Err(ACPI_Error::NotMapped);
		}
		let header = unsafe{ &*(entry as *const header) };
		debug!("{:?}", header);
		if &header.Signature == b"APIC" {
			madt = Some(unsafe{ &*(entry as *const madt) });
		}
	}
	debug!("{:?}", madt);
	// return acpi_config_smp(madt);
	Ok(())
}
