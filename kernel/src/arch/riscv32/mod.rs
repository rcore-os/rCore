extern crate bbl;
extern crate riscv;

pub mod io;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;

pub fn init() {
    println!("Hello RISCV! {}", 123);
    interrupt::init();
    memory::init();
    timer::init();
}

global_asm!(include_str!("boot/entry.asm"));
global_asm!(include_str!("boot/trap.asm"));