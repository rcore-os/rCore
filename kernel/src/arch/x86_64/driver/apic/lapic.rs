extern {
	//noinspection RsStaticConstNaming
	static mut lapic: *const ();
	fn lapicinit();	// must set `lapic` first
	fn lapiceoi();	// ack
	fn lapicstartap(apicid: u8, addr: u32);
}

pub fn set_addr(lapic_addr: usize) {
	unsafe {
//		lapic = lapic_addr;
	}
}

pub fn init() {
    warn!("lapic::init use C lib");
	unsafe {
//		lapicinit();
	}
    info!("lapic: init end");
}

pub fn ack(_irq: u8) {
	unsafe {
//		lapiceoi();
	}
}

pub fn start_ap(apicid: u8, addr: u32) {
    warn!("lapic::start_ap use C lib");
	unsafe {
//		lapicstartap(apicid, addr);
	}
}

pub fn lapic_id() -> u8 {
	0
//	unsafe{
//        if lapic.is_null() {
//            warn!("lapic is null. return lapic id = 0");
//            return 0;
//        }
//        let ptr = (lapic as *const u32).offset(0x0020 / 4);
//        (ptr.read_volatile() >> 24) as u8
//	}
}