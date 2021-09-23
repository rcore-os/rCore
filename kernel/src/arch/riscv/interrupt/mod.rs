use crate::arch::interrupt::consts::SupervisorExternal;
use crate::drivers::IRQ_MANAGER;
use crate::process::thread::Thread;
use alloc::sync::Arc;
use log::*;
use riscv::register::*;
use riscv::register::{scause::Scause, sscratch, stvec};
use trapframe::{TrapFrame, UserContext};

pub mod consts;

/// Enable interrupt
#[inline]
pub unsafe fn enable() {
    sstatus::set_sie();
}

/// Disable interrupt and return current interrupt status
#[inline]
pub unsafe fn disable_and_store() -> usize {
    let e = sstatus::read().sie() as usize;
    sstatus::clear_sie();
    e
}

/// Enable interrupt if `flags` != 0
#[inline]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        enable();
    }
}

/// Dispatch and handle interrupt.
///
/// This function is called from `trap.asm`.
#[no_mangle]
pub extern "C" fn trap_handler(tf: &mut TrapFrame) {
    trap_handler_no_frame(&mut tf.sepc);
}

use crate::memory::AccessType;
#[inline]
pub fn trap_handler_no_frame(sepc: &mut usize) {
    use self::scause::{Exception as E, Interrupt as I, Trap};
    let scause = scause::read();
    let stval = stval::read();
    let is_user = false;
    trace!("Interrupt @ CPU{}: {:?} ", super::cpu::id(), scause.cause());
    match scause.cause() {
        Trap::Interrupt(I::SupervisorExternal) => external(),
        Trap::Interrupt(I::SupervisorSoft) => ipi(),
        Trap::Interrupt(I::SupervisorTimer) => timer(),
        Trap::Exception(E::LoadPageFault) => page_fault(stval, sepc, AccessType::read(is_user)),
        Trap::Exception(E::StorePageFault) => page_fault(stval, sepc, AccessType::write(is_user)),
        Trap::Exception(E::InstructionPageFault) => {
            page_fault(stval, sepc, AccessType::execute(is_user))
        }
        _ => {
            let bits = scause.bits();
            panic!("unhandled trap {:?} ({})", scause.cause(), bits);
        }
    }
    trace!("Interrupt end");
}

fn external() {
    #[cfg(feature = "board_u540")]
    unsafe {
        super::board::handle_external_interrupt();
    }
    IRQ_MANAGER
        .read()
        .try_handle_interrupt(Some(SupervisorExternal));
}

fn ipi() {
    debug!("IPI");
    super::sbi::clear_ipi();
}

pub fn timer() {
    super::timer::set_next();
    crate::trap::timer();
}

fn page_fault(stval: usize, sepc: &mut usize, access: AccessType) {
    let addr = stval;
    info!("\nEXCEPTION: Page Fault @ {:#x}", addr);

    if crate::memory::handle_page_fault_ext(addr, access) {
        return;
    }
    extern "C" {
        fn _copy_user_start();
        fn _copy_user_end();
    }
    if *sepc >= _copy_user_start as usize && *sepc < _copy_user_end as usize {
        info!("fixup for addr {:x?}", addr);
        *sepc = crate::memory::read_user_fixup as usize;
        return;
    }
    error!("unhandled page fault {:#x} from {:#x}", addr, sepc);
    panic!("unhandled page fault");
}

pub fn ack(irq: usize) {
    // Handled in PLIC driver
}

pub fn enable_irq(irq: usize) {
    // Handled in PLIC driver
}

pub fn get_trap_num(_context: &UserContext) -> usize {
    scause::read().bits()
}

pub fn wait_for_interrupt() {
    unsafe {
        // enable interrupt and disable
        let sie = riscv::register::sstatus::read().sie();
        riscv::register::sstatus::set_sie();
        riscv::asm::wfi();
        if !sie {
            riscv::register::sstatus::clear_sie();
        }
    }
}

pub fn handle_user_page_fault_ext(thread: &Arc<Thread>, addr: usize, access: AccessType) -> bool {
    thread.vm.lock().handle_page_fault_ext(addr, access)
}

pub fn handle_reserved_inst(tf: &mut UserContext) -> bool {
    false
}
