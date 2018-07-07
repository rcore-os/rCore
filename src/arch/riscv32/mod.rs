extern crate riscv;
extern crate bbl;

pub mod serial;
pub mod interrupt;

pub fn init() {
    println!("Hello RISCV! {}", 123);
    interrupt::init();
    // Trigger interrupt
    unsafe { asm!("mret"); }
    loop {}
}