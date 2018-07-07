use super::riscv::register::*;

pub fn init() {
    unsafe {
        // Set the exception vector address
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        // Enable interrupt
        sstatus::set_sie();
    }
    println!("interrupt: init end");
}

#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) {
    use super::riscv::register::scause::{Trap, Interrupt, Exception};
    match tf.scause.cause() {
        Trap::Interrupt(SupervisorTimer) => timer(),
        _ => panic!("Unhandled interrupt: {:?}\n{:#x?}", tf.scause.cause(), tf),
    }
}

fn timer() {
    static mut TICK: usize = 0;
    unsafe {
        TICK += 1;
        if TICK % 100 == 0 {
            println!("timer");
        }
    }
    super::timer::set_next();
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct TrapFrame {
    x: [usize; 32],
    sstatus: sstatus::Sstatus,
    sepc: usize,
    sbadaddr: usize,
    scause: scause::Scause,
}

extern {
    fn __alltraps();
}