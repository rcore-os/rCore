extern crate bbl;
extern crate riscv;

pub mod serial;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;

pub fn init() {
    println!("Hello RISCV! {}", 123);
    interrupt::init();
    memory::init();
//    timer::init();
    loop {}
}