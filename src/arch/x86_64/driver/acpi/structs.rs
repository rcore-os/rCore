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
    pub Signature: [u8; 4],
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
    TableOffsetEntry: [u32; 0],
}

impl rsdt {
    pub unsafe fn entry_at(&self, id: usize) -> u32 {
        *(self.TableOffsetEntry.as_ptr().offset(id as isize))
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct madt {
    pub Header: header,
    pub Address: u32,
    pub Flags: u32,
    Table: [u32; 0],
}

impl Checkable for madt {
    fn check(&self) -> bool {
        &self.Header.Signature == b"APIC"
    }
}

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