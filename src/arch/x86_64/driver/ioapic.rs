// Migrate from xv6 ioapic.c

/// The I/O APIC manages hardware interrupts for an SMP system.
/// http://www.intel.com/design/chipsets/datashts/29056601.pdf
/// See also picirq.c.

use core::ptr::{read_volatile, write_volatile};

pub fn init() {

}

const IOAPIC_ADDRESS  : u32 = 0xFEC00000;   // Default physical address of IO APIC

const REG_ID     : u32 = 0x00;  // Register index: ID
const REG_VER    : u32 = 0x01;  // Register index: version
const REG_TABLE  : u32 = 0x10;  // Redirection table base

// The redirection table starts at REG_TABLE and uses
// two registers to configure each interrupt.
// The first (low) register in a pair contains configuration bits.
// The second (high) register contains a bitmask telling which
// CPUs can serve that interrupt.
const INT_DISABLED   : u32 = 0x00010000;  // Interrupt disabled
const INT_LEVEL      : u32 = 0x00008000;  // Level-triggered (vs edge-)
const INT_ACTIVELOW  : u32 = 0x00002000;  // Active low (vs high)
const INT_LOGICAL    : u32 = 0x00000800;  // Destination is CPU id (vs APIC ID)

// static IOAPIC: *mut IoApic = IOAPIC_ADDRESS as *mut _;

const ioapicid: u32 = 0; // TODO fix
const T_IRQ0: u32 = 32;

// IO APIC MMIO structure: write reg, then read or write data.
#[repr(C)]
struct IoApic {
	reg: u32,
	pad: [u32; 3],
	data: u32,
}

impl IoApic {
	unsafe fn read(&mut self, reg: u32) -> u32
	{
		write_volatile(&mut self.reg as *mut _, reg);
		read_volatile(&self.data as *const _)
	}
	unsafe fn write(&mut self, reg: u32, data: u32)
	{
		write_volatile(&mut self.reg as *mut _, reg);
		write_volatile(&mut self.data as *mut _, data);
	}
	unsafe fn init(&mut self)
	{
		let maxintr = (self.read(REG_VER) >> 16) & 0xFF;
		let id = self.read(REG_ID) >> 24;
		if id != ioapicid {
			println!("ioapicinit: id isn't equal to ioapicid; not a MP");
		}

		// Mark all interrupts edge-triggered, active high, disabled,
		// and not routed to any CPUs.
		for i in 0 .. maxintr+1 {
			self.write(REG_TABLE+2*i, INT_DISABLED | (T_IRQ0 + i));
			self.write(REG_TABLE+2*i+1, 0);
		}
	}
	unsafe fn enable(&mut self, irq: u32, cpunum: u32)
	{
		// Mark interrupt edge-triggered, active high,
		// enabled, and routed to the given cpunum,
		// which happens to be that cpu's APIC ID.
		self.write(REG_TABLE+2*irq, T_IRQ0 + irq);
		self.write(REG_TABLE+2*irq+1, cpunum << 24);
	}
}