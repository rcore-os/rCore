extern {
	static mut lapic: *const ();
	fn lapicinit();	// must set `lapic` first
}

pub fn init(lapic_addr: *const ()) {
	debug!("WARNING: lapic::init use C lib");	
	unsafe {
		lapic = lapic_addr;
		debug!("lapic = {:?}", lapic);
		unimplemented!();
		lapicinit();
	}	
}