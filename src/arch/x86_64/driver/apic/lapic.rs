extern {
	static mut lapic: *const ();
	fn lapicinit();	// must set `lapic` first
	fn lapiceoi();	// ack
	fn lapicstartap(apicid: u8, addr: u32);
}

pub fn set_addr(lapic_addr: *const ()) {
	unsafe {
		lapic = lapic_addr;
	}
}

pub fn init() {
	debug!("WARNING: lapic::init use C lib");
	unsafe {
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

pub fn lapic_id() -> u8 {
	unsafe{
		(*(lapic as *const u32).offset(0x0020/4) >> 24) as u8
	}
}