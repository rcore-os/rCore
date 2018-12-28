#[cfg(feature = "m_mode")]
use riscv::register::{
    mstatus as xstatus,
    mscratch as xscratch,
    mtvec as xtvec,
};
#[cfg(not(feature = "m_mode"))]
use riscv::register::{
    sstatus as xstatus,
    sscratch as xscratch,
    stvec as xtvec,
};
use riscv::register::{mcause, mepc, sie, mie};
pub use self::context::*;
use log::*;

#[path = "context.rs"]
mod context;

/*
* @brief:
*   initialize the interrupt status
*/
pub fn init() {
    extern {
        fn __alltraps();
    }
    unsafe {
        // Set sscratch register to 0, indicating to exception vector that we are
        // presently executing in the kernel
        xscratch::write(0);
        // Set the exception vector address
        xtvec::write(__alltraps as usize, xtvec::TrapMode::Direct);
        // Enable IPI
        sie::set_ssoft();
        // Enable serial interrupt
        #[cfg(feature = "m_mode")]
        mie::set_mext();
        #[cfg(not(feature = "m_mode"))]
        sie::set_sext();
        // NOTE: In M-mode: mie.MSIE is set by BBL.
        //                  mie.MEIE can not be set in QEMU v3.0
        //                  (seems like a bug)
    }
    info!("interrupt: init end");
}

/*
* @brief:
*   enable interrupt
*/
#[inline(always)]
pub unsafe fn enable() {
    xstatus::set_xie();
}

/*
* @brief:
*   store and disable interrupt
* @retbal:
*   a usize value store the origin sie
*/
#[inline(always)]
pub unsafe fn disable_and_store() -> usize {
    let e = xstatus::read().xie() as usize;
    xstatus::clear_xie();
    e
}

/*
* @param:
*   flags: input flag
* @brief:
*   enable interrupt if flags != 0
*/
#[inline(always)]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        xstatus::set_xie();
    }
}

/*
* @param:
*   TrapFrame: the trapFrame of the Interrupt/Exception/Trap to be processed
* @brief:
*   process the Interrupt/Exception/Trap
*/
#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) {
    use self::mcause::{Trap, Interrupt as I, Exception as E};
    trace!("Interrupt @ CPU{}: {:?} ", super::cpu::id(), tf.scause.cause());
    match tf.scause.cause() {
        // M-mode only
        Trap::Interrupt(I::MachineExternal) => serial(),
        Trap::Interrupt(I::MachineSoft) => ipi(),
        Trap::Interrupt(I::MachineTimer) => timer(),
        Trap::Exception(E::MachineEnvCall) => sbi(tf),

        Trap::Interrupt(I::SupervisorExternal) => serial(),
        Trap::Interrupt(I::SupervisorSoft) => ipi(),
        Trap::Interrupt(I::SupervisorTimer) => timer(),
        Trap::Exception(E::IllegalInstruction) => illegal_inst(tf),
        Trap::Exception(E::UserEnvCall) => syscall(tf),
        Trap::Exception(E::LoadPageFault) => page_fault(tf),
        Trap::Exception(E::StorePageFault) => page_fault(tf),
        Trap::Exception(E::InstructionPageFault) => page_fault(tf),
        _ => crate::trap::error(tf),
    }
    trace!("Interrupt end");
}

/// Call BBL SBI functions for M-mode kernel
fn sbi(tf: &mut TrapFrame) {
    (super::BBL.mcall_trap)(tf.x.as_ptr(), tf.scause.bits(), tf.sepc);
    tf.sepc += 4;
}

fn serial() {
    crate::trap::serial(super::io::getchar());
}

fn ipi() {
    debug!("IPI");
    bbl::sbi::clear_ipi();
}

/*
* @brief:
*   process timer interrupt
*/
fn timer() {
    super::timer::set_next();
    crate::trap::timer();
}

/*
* @param:
*   TrapFrame: the Trapframe for the syscall
* @brief:
*   process syscall
*/
fn syscall(tf: &mut TrapFrame) {
    tf.sepc += 4;   // Must before syscall, because of fork.
    let ret = crate::syscall::syscall(tf.x[10], [tf.x[11], tf.x[12], tf.x[13], tf.x[14], tf.x[15], tf.x[16]], tf);
    tf.x[10] = ret as usize;
}

/*
* @param:
*   TrapFrame: the Trapframe for the illegal inst exception
* @brief:
*   process IllegalInstruction exception
*/
fn illegal_inst(tf: &mut TrapFrame) {
    (super::BBL.illegal_insn_trap)(tf.x.as_ptr(), tf.scause.bits(), tf.sepc);
    tf.sepc = mepc::read();
}

/*
* @param:
*   TrapFrame: the Trapframe for the page fault exception
* @brief:
*   process page fault exception
*/
fn page_fault(tf: &mut TrapFrame) {
    let addr = tf.stval;
    trace!("\nEXCEPTION: Page Fault @ {:#x}", addr);

    if !crate::memory::page_fault_handler(addr) {
        crate::trap::error(tf);
    }
}
