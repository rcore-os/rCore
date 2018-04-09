// Rust language features implementions

use core;
use arch::cpu;

#[lang = "eh_personality"] 
extern fn eh_personality() {
}

#[lang = "panic_fmt"]
#[no_mangle]
pub extern fn panic_fmt(fmt: core::fmt::Arguments, file: &'static str, line: u32) -> ! {
    println!("\n\nPANIC in {} at line {}:", file, line);
    println!("    {}", fmt);
    if cfg!(feature = "qemu_auto_exit") {
        unsafe{ cpu::exit_in_qemu(1) }
    } else {
        loop { }
    }
}
