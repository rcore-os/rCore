extern crate riscv;
extern crate bbl;

pub mod serial;
pub mod interrupt;
pub mod timer;

pub fn init() {
    println!("Hello RISCV! {}", 123);
    interrupt::init();
    timer::init();
    loop {}
}