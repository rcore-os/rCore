// Migrate from xv6-x86_64 acpi.c

mod structs;
use self::structs::*;
use consts::*;

pub fn init() -> Result<ACPI_Result, ACPI_Error> {
	let rsdp = find_rsdp().expect("acpi: rsdp not found.");
	if rsdp.RsdtPhysicalAddress > PHYSICAL_MEMORY_LIMIT {
		return Err(ACPI_Error::NotMapped);
	}
	debug!("RSDT at {:#x}", rsdp.RsdtPhysicalAddress);
	let rsdt = unsafe{ &*(rsdp.RsdtPhysicalAddress as *const rsdt) };
	let mut madt: Option<&'static madt> = None;
	for i in 0 .. rsdt.entry_count() {
		let entry = rsdt.entry_at(i);
		if entry > PHYSICAL_MEMORY_LIMIT {
			return Err(ACPI_Error::NotMapped);
		}
		let header = unsafe{ &*(entry as *const header) };
		// debug!("{:?}", header);
		if &header.Signature == b"APIC" {
			madt = Some(unsafe{ &*(entry as *const madt) });
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
pub struct ACPI_Result {
	pub cpu_num: u8,
	pub cpu_acpi_ids: [u8; MAX_CPU_NUM],
	pub ioapic_id: u8,
	pub lapic_addr: *const (),
}

#[derive(Debug)]
pub enum ACPI_Error {
	NotMapped,
	IOACPI_NotFound,
}

fn config_smp(madt: &'static madt) -> Result<ACPI_Result, ACPI_Error> {
	let lapic_addr = madt.LapicAddress as *const ();

	let mut cpu_num = 0u8;
	let mut cpu_acpi_ids: [u8; MAX_CPU_NUM] = [0; MAX_CPU_NUM];
	let mut ioapic_id: Option<u8> = None;
	for entry in madt.entry_iter() {
		debug!("{:?}", entry);
		match &entry {
			&MadtEntry::LocalApic(ref lapic) => {
				cpu_acpi_ids[cpu_num as usize] = lapic.Id;
				cpu_num += 1;
			},
			&MadtEntry::IoApic(ref ioapic) => {
				ioapic_id = Some(ioapic.Id);
			},
			_ => {},
		}
	}

	if ioapic_id.is_none() {
		return Err(ACPI_Error::IOACPI_NotFound);
	}
	let ioapic_id = ioapic_id.unwrap();
	Ok(ACPI_Result { cpu_num, cpu_acpi_ids, ioapic_id, lapic_addr })
}

/// See https://wiki.osdev.org/RSDP -- Detecting the RSDP
fn find_rsdp() -> Option<&'static rsdp> {
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