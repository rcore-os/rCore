// Migrate from xv6 mp.h

// See MultiProcessor Specification Version 1.[14]

use util::{Checkable, bytes_sum};

#[repr(C)]
#[derive(Debug)]
pub struct MP {             // floating pointer
	signature: [u8; 4],         // "_MP_"
	physaddr: u32,               // phys addr of MP config table
	length: u8,                 // 1
	specrev: u8,                // [14]
	checksum: u8,               // all bytes must add up to 0
	type_: u8,                   // MP system config type
	imcrp: u8,
	reserved: [u8; 3],
}

impl Checkable for MP {
	fn check(&self) -> bool {
		&self.signature == b"_MP_" && bytes_sum(self) == 0
	}
}

/*

#[repr(C)]
struct mpconf {         // configuration table header
	signature: [byte; 4];           // "PCMP"
	length: u16,                // total table length
	version: u8,                // [14]
	checksum: u8,               // all bytes must add up to 0
	product: [u8; 20],            // product id
	uint *oemtable,               // OEM table pointer
	ushort oemlength,             // OEM table length
	ushort entry,                 // entry count
	uint *lapicaddr,              // address of local APIC
	ushort xlength,               // extended table length
	u8 xchecksum,              // extended table checksum
	u8 reserved,
}

#[repr(C)]
struct mpproc {         // processor table entry
	u8 type;                   // entry type (0)
	u8 apicid;                 // local APIC id
	u8 version;                // local APIC verison
	u8 flags;                  // CPU flags
		#define MPBOOT 0x02           // This proc is the bootstrap processor.
	u8 signature[4];           // CPU signature
	uint feature;                 // feature flags from CPUID instruction
	u8 reserved[8];
}

struct mpioapic {       // I/O APIC table entry
	u8 type;                   // entry type (2)
	u8 apicno;                 // I/O APIC id
	u8 version;                // I/O APIC version
	u8 flags;                  // I/O APIC flags
	uint *addr;                  // I/O APIC address
}

// Table entry types
const MPPROC    : u8 = 0x00;  // One per processor
const MPBUS     : u8 = 0x01;  // One per bus
const MPIOAPIC  : u8 = 0x02;  // One per I/O APIC
const MPIOINTR  : u8 = 0x03;  // One per bus interrupt source
const MPLINTR   : u8 = 0x04;  // One per system interrupt source

//PAGEBREAK!
// Blank page.

*/
