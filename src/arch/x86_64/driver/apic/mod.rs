pub use self::ioapic::IOAPIC;
pub use self::lapic::ack;

mod lapic;
mod ioapic;

pub fn init(lapic_addr: *const (), ioapic_id: u8) {
	assert_has_not_been_called!("apic::init must be called only once");
	self::lapic::init(lapic_addr);
	self::ioapic::init(ioapic_id);
}