use syscall::sys_exit;
use core::alloc::Layout;
use core::panic::PanicInfo;

#[linkage = "weak"]
#[no_mangle]
fn main() {
    panic!("No main() linked");
}

#[no_mangle]
pub extern fn _start(_argc: isize, _argv: *const *const u8) -> ! {
    main();
    sys_exit(0)
}

#[lang = "eh_personality"]
fn eh_personality() {}

#[panic_implementation]
fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    println!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    sys_exit(1)
}

#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}

#[no_mangle]
pub extern fn abort() -> ! {
    sys_exit(2)
}

#[no_mangle]
pub extern fn __mulsi3(mut a: u32, mut b: u32) -> u32 {
    let mut r: u32 = 0;

    while a > 0 {
        if a & 1 > 0 {
            r += b;
        }
        a >>= 1;
        b <<= 1;
    }

    r
}