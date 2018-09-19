pub fn init() {
	assert_has_not_been_called!("keyboard::init must be called only once");

	use arch::interrupt::consts::*;
	use arch::interrupt::enable_irq;
	enable_irq(IRQ_KBD);
}

pub fn get() -> i32 {
	0
}

extern {
    fn kbdgetc() -> i32;
}