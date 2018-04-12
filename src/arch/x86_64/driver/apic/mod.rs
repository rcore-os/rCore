mod lapic;
mod ioapic;

pub fn init(lapic_addr: *const ()) {
	self::lapic::init(lapic_addr);
	// self::ioapic::init();
}