// Migrate from xv6-x86_64 acpi.c

mod structs;
use self::structs::*;
use consts::*;

pub fn init() -> Result<AcpiResult, AcpiError> {
	let rsdp = find_rsdp().expect("acpi: rsdp not found.");
	if rsdp.rsdt_physical_address > PHYSICAL_MEMORY_LIMIT {
		return Err(AcpiError::NotMapped);
	}
	debug!("RSDT at {:#x}", rsdp.rsdt_physical_address);
	let rsdt = unsafe{ &*(rsdp.rsdt_physical_address as *const Rsdt) };
	let mut madt: Option<&'static Madt> = None;
	for i in 0 .. rsdt.entry_count() {
		let entry = rsdt.entry_at(i);
		if entry > PHYSICAL_MEMORY_LIMIT {
			return Err(AcpiError::NotMapped);
		}
		let header = unsafe{ &*(entry as *const Header) };
		// debug!("{:?}", header);
		if &header.signature == b"APIC" {
			madt = Some(unsafe{ &*(entry as *const Madt) });
		}
	}
	debug!("{:?}", madt);
	config_smp(madt.expect("acpi: madt not found."))
}

#[cfg(target_arch="x86")]
const PHYSICAL_MEMORY_LIMIT: u32 = 0x0E000000;
#[cfg(target_arch="x86_64")]
const PHYSICAL_MEMORY_LIMIT: u32 = 0x80000000;

#[derive(Debug)]
pub struct AcpiResult {
	pub cpu_num: u8,
	pub cpu_acpi_ids: [u8; MAX_CPU_NUM],
	pub ioapic_id: u8,
	pub lapic_addr: *const (),
}

#[derive(Debug)]
pub enum AcpiError {
	NotMapped,
	IoacpiNotFound,
}

fn config_smp(madt: &'static Madt) -> Result<AcpiResult, AcpiError> {
	let lapic_addr = madt.lapic_address as *const ();

	let mut cpu_num = 0u8;
	let mut cpu_acpi_ids: [u8; MAX_CPU_NUM] = [0; MAX_CPU_NUM];
	let mut ioapic_id: Option<u8> = None;
	for entry in madt.entry_iter() {
		debug!("{:?}", entry);
		match &entry {
			&MadtEntry::LocalApic(ref lapic) => {
				cpu_acpi_ids[cpu_num as usize] = lapic.id;
				cpu_num += 1;
			},
			&MadtEntry::IoApic(ref ioapic) => {
				ioapic_id = Some(ioapic.id);
			},
			_ => {},
		}
	}

	if ioapic_id.is_none() {
		return Err(AcpiError::IoacpiNotFound);
	}
	let ioapic_id = ioapic_id.unwrap();
	Ok(AcpiResult { cpu_num, cpu_acpi_ids, ioapic_id, lapic_addr })
}

/// See https://wiki.osdev.org/RSDP -- Detecting the RSDP
fn find_rsdp() -> Option<&'static Rsdp> {
	use util::{Checkable, find_in_memory};
	let ebda = unsafe { *(0x40E as *const u16) as usize } << 4;
	debug!("EBDA at {:#x}", ebda);

	macro_rules! return_if_find_in {
		($begin:expr, $end:expr) => (
			if let Some(addr) = unsafe{ find_in_memory::<Rsdp>($begin, $end, 4) } {
				return Some(unsafe{ &*(addr as *const Rsdp) });
			}
		)
	}
	
	if ebda != 0 {
		return_if_find_in!(ebda as usize, 1024);
	}
	return_if_find_in!(0xE0000, 0x20000);
	None
} 