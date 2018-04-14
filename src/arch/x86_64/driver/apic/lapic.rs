extern {
	static mut lapic: *const ();
	fn lapicinit();	// must set `lapic` first
}

pub unsafe fn init(lapic_addr: *const ()) {
	debug!("WARNING: lapic::init use C lib");	
	lapic = lapic_addr;
	lapicinit();
}