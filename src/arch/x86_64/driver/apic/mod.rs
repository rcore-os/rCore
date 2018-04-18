pub use self::ioapic::IOAPIC;
pub use self::lapic::{ack, start_ap, lapic_id};

mod lapic;
mod ioapic;

pub fn init(lapic_addr: *const (), ioapic_id: u8) {
	assert_has_not_been_called!("apic::init must be called only once");
	self::lapic::set_addr(lapic_addr);
	self::lapic::init();
	self::ioapic::init(ioapic_id);
}

pub fn other_init() {
	self::lapic::init();
}