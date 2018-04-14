extern {
	static mut lapic: *const ();
	fn lapicinit();	// must set `lapic` first
	fn lapiceoi();	// ack
}

pub fn init(lapic_addr: *const ()) {
	debug!("WARNING: lapic::init use C lib");
	unsafe {
		lapic = lapic_addr;
		lapicinit();
	}
	debug!("lapic: init end");
}

pub fn ack(_irq: u8) {
	unsafe {
		lapiceoi();
	}
}