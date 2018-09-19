extern crate riscv;
extern crate bbl;

pub mod io;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;

#[no_mangle]
pub extern fn rust_main() -> ! {
    println!("Hello RISCV! {}", 123);
    ::logging::init();
    interrupt::init();
    memory::init();
    timer::init();
    ::kmain();
}

#[cfg(feature = "no_bbl")]
global_asm!(include_str!("boot/boot.asm"));
global_asm!(include_str!("boot/entry.asm"));
global_asm!(include_str!("boot/trap.asm"));