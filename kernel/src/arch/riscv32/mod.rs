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
    // First init log mod, so that we can print log info.
    ::logging::init();
    // Init interrupt handling.
    interrupt::init();
    // Init physical memory management and heap
    memory::init();
    // Init timer interrupt
    timer::init();
    
    ::kmain();
}

#[cfg(feature = "no_bbl")]
global_asm!(include_str!("boot/boot.asm"));
global_asm!(include_str!("boot/entry.asm"));
global_asm!(include_str!("boot/trap.asm"));