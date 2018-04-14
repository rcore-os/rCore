pub fn init() {
	use consts::irq::IRQ_KBD;
	// TODO set irq handler
	// super::pic::enable_irq(IRQ_KBD);
	super::apic::IOAPIC.lock().enable(IRQ_KBD, 0);
}