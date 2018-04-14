mod lapic;
mod ioapic;

pub unsafe fn init(lapic_addr: *const (), ioapic_id: u8) {
	self::lapic::init(lapic_addr);
	self::ioapic::init(ioapic_id);
}