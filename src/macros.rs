macro_rules! test_end {
	() => (
		println!("Test end");
		// test success	
		unsafe{ arch::cpu::exit_in_qemu(11) }
	)
}

macro_rules! test {
	// ($name:expr, $body:expr) => (
	// 	if cfg!(feature = "test") {
	// 		println!("Testing: {}", $name);
	// 		$body;
	// 		println!("Success: {}", $name);
	// 	}
	// );
	($func:ident) => (
		if cfg!(feature = "test") {
			println!("Testing: {}", stringify!($func));
			test::$func();
			println!("Success: {}", stringify!($func));
		}
	)
}
