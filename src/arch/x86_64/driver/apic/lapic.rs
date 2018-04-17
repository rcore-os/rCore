extern {
	static mut lapic: *const ();
	fn lapicinit();	// must set `lapic` first
	fn lapiceoi();	// ack
	fn lapicstartap(apicid: u8, addr: u32);
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

pub fn start_ap(apicid: u8, addr: u32) {
	debug!("WARNING: lapic::start_ap use C lib");
	unsafe {
		lapicstartap(apicid, addr);
	}
}