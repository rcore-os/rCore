// Rust language features implementions

use core::panic::PanicInfo;

#[lang = "eh_personality"] 
extern fn eh_personality() {
}

#[cfg(target_arch = "x86_64")]
#[panic_implementation]
#[no_mangle]
pub fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    error!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    if cfg!(feature = "qemu_auto_exit") {
        use arch::cpu;
        unsafe{ cpu::exit_in_qemu(3) }
    } else {
        loop { }
    }
}

#[cfg(target_arch = "riscv")]
#[lang = "panic_fmt"]
unsafe extern "C" fn panic_fmt(_fmt: ::core::fmt::Arguments, _file: &'static str, _line: u32, _col: u32) -> ! {
    loop {}
}

#[cfg(target_arch = "x86_64")]
#[lang = "oom"]
#[no_mangle]
fn oom() -> ! {
    panic!("out of memory");
}
