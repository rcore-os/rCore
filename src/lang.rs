// Rust language features implementions

use core::panic::PanicInfo;
use arch::cpu;

#[lang = "eh_personality"] 
extern fn eh_personality() {
}

#[panic_implementation]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    error!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    if cfg!(feature = "qemu_auto_exit") {
        unsafe{ cpu::exit_in_qemu(3) }
    } else {
        loop { }
    }
}

#[lang = "oom"]
#[no_mangle]
fn oom() -> ! {
    panic!("out of memory");
}
