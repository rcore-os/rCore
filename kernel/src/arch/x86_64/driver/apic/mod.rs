pub use self::ioapic::IOAPIC;
pub use self::lapic::{ack, start_ap, lapic_id};

mod lapic;
mod ioapic;

pub fn init() {
	assert_has_not_been_called!("apic::init must be called only once");
	use consts::KERNEL_OFFSET;
	self::lapic::set_addr(KERNEL_OFFSET + 0xfee00000);
	self::lapic::init();
	self::ioapic::init();
}

pub fn other_init() {
	self::lapic::init();
}