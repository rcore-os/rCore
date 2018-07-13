use syscall::sys_exit;

#[linkage = "weak"]
#[no_mangle]
fn main() {
    panic!("No main() linked");
}

#[no_mangle]
pub extern fn _start(_argc: isize, _argv: *const *const u8) -> !
{
    main();
    sys_exit(0)
}

#[lang = "eh_personality"]
fn eh_personality() {}

#[cfg(target_arch = "x86_64")]
#[panic_implementation]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    println!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    sys_exit(1)
}

#[cfg(target_arch = "riscv")]
#[lang = "panic_fmt"]
#[no_mangle]
pub fn panic_fmt(fmt: ::core::fmt::Arguments, file: &'static str, line: u32, col: u32) -> ! {
    println!("\n\nPANIC in {} at {}:{}\n    {}", file, line, col, fmt);
    sys_exit(1)
}

#[cfg(target_arch = "x86_64")]
#[lang = "oom"]
fn oom() -> ! {
    panic!("out of memory");
}