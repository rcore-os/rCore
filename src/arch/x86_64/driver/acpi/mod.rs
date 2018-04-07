// Migrate from xv6-x86_64 acpi.c

mod structs;
use self::structs::*;

/// See https://wiki.osdev.org/RSDP -- Detecting the RSDP
pub fn find_rdsp() -> Option<*const rsdp> {
	use util::{Checkable, find_in_memory};
	let ebda = unsafe { *(0x40E as *const u16) as usize } << 4;
	debug!("EBDA at {:#x}", ebda);
	if ebda != 0 {
		if let Some(addr) = unsafe{ find_in_memory::<rsdp>(ebda as usize, 1024, 4) } {
			return Some(addr as *const rsdp);
		}
	}
	unsafe{ find_in_memory::<rsdp>(0xE0000, 0x20000, 4).map(|addr| addr as *const rsdp) }
} 

