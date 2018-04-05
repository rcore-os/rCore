// Reference: xv6-x86_64 acpi.h
// Copy from crate 'acpica-sys'

use util::{Checkable, bytes_sum};

#[repr(C)]
#[derive(Debug)]
pub struct rsdp {
    pub Signature: [u8; 8],
    pub Checksum: u8,
    pub OemId: [i8; 6],
    pub Revision: u8,
    pub RsdtPhysicalAddress: u32,
    pub Length: u32,
    pub XsdtPhysicalAddress: u64,
    pub ExtendedChecksum: u8,
    pub Reserved: [u8; 3],
}

impl Checkable for rsdp {
    fn check(&self) -> bool {
        &self.Signature == b"RSD PTR " && bytes_sum(self) == 0
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct header {
    pub Signature: [i8; 4],
    pub Length: u32,
    pub Revision: u8,
    pub Checksum: u8,
    pub OemId: [i8; 6],
    pub OemTableId: [i8; 8],
    pub OemRevision: u32,
    pub AslCompilerId: [i8; 4],
    pub AslCompilerRevision: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct rsdt {
    pub Header: header,
    pub TableOffsetEntry: [u32; 1],
}

#[repr(C)]
#[derive(Debug)]
pub struct madt {
    pub Header: header,
    pub Address: u32,
    pub Flags: u32,
}

const MADT_SIGNATURE: [u8; 4] = *b"APIC";

#[repr(C)]
#[derive(Debug)]
pub struct madt_local_apic {
	pub Type: u8,
    pub Length: u8,
    pub ProcessorId: u8,
    pub Id: u8,
    pub LapicFlags: u32,
}

#[repr(C)]
#[derive(Debug)]
pub struct madt_io_apic {
    pub Type: u8,
    pub Length: u8,
    pub Id: u8,
    pub Reserved: u8,
    pub Address: u32,
    pub GlobalIrqBase: u32,
}