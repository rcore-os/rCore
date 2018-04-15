use x86_64;

pub fn enable() {
    unsafe{ x86_64::instructions::interrupts::enable(); }	
}

pub fn disable() {
    unsafe{ x86_64::instructions::interrupts::disable(); }	
}