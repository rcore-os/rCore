pub fn init() {
	use consts::irq::{IRQ_KBD, IRQ_COM1};
	// TODO set irq handler
	// super::pic::enable_irq(IRQ_KBD);
	let mut ioapic = super::apic::IOAPIC.lock();
	ioapic.enable(IRQ_KBD, 0);
	ioapic.enable(IRQ_COM1, 0);
}