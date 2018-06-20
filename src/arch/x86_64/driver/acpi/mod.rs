// Migrate from xv6-x86_64 acpi.c

mod structs;
use self::structs::*;
use consts::*;

pub fn init(rsdt_addr: usize) -> Result<AcpiResult, AcpiError> {
	let rsdt = unsafe { &*(rsdt_addr as *const Rsdt) };
	let mut madt: Option<&'static Madt> = None;
	for i in 0 .. rsdt.entry_count() {
		let entry = rsdt.entry_at(i);
		let header = unsafe{ &*(entry as *const Header) };
        trace!("{:?}", header);
		if &header.signature == b"APIC" {
			madt = Some(unsafe{ &*(entry as *const Madt) });
		}
	}
    trace!("{:?}", madt);
	config_smp(madt.expect("acpi: madt not found."))
}

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
        trace!("{:?}", entry);
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
