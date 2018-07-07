use super::riscv::register::*;

pub fn init() {
    unsafe {
        // Set the exception vector address
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        // Enable interrupt
        sstatus::set_sie();
    }
}

#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) {
    println!("Trap:\n{:#x?}", tf);
    loop {}
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