// Rust language features implementions

use core::panic::PanicInfo;
use core::alloc::Layout;

#[lang = "eh_personality"] 
extern fn eh_personality() {
}

#[panic_handler]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    error!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    loop { }
}

#[lang = "oom"]
#[no_mangle]
pub fn oom(_: Layout) -> ! {
    panic!("out of memory");
}
