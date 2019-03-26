use riscv::register::*;
use crate::drivers::DRIVERS;
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
        fn trap_entry();
    }
    unsafe {
        // Set sscratch register to 0, indicating to exception vector that we are
        // presently executing in the kernel
        sscratch::write(0);
        // Set the exception vector address
        stvec::write(trap_entry as usize, stvec::TrapMode::Direct);
        // Enable IPI
        sie::set_ssoft();
        // Enable external interrupt
        if super::cpu::id() == super::BOOT_HART_ID {
            sie::set_sext();
        }
    }
    info!("interrupt: init end");
}

/*
* @brief:
*   enable interrupt
*/
#[inline]
pub unsafe fn enable() {
    sstatus::set_sie();
}

#[inline]
pub unsafe fn disable_and_store() -> usize {
    let e = sstatus::read().sie() as usize;
    sstatus::clear_sie();
    e
}

/*
* @param:
*   flags: input flag
* @brief:
*   enable interrupt if flags != 0
*/
#[inline]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        enable();
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
    use self::scause::{Trap, Interrupt as I, Exception as E};
    trace!("Interrupt @ CPU{}: {:?} ", super::cpu::id(), tf.scause.cause());
    match tf.scause.cause() {
        Trap::Interrupt(I::SupervisorExternal) => external(),
        Trap::Interrupt(I::SupervisorSoft) => ipi(),
        Trap::Interrupt(I::SupervisorTimer) => timer(),
        Trap::Exception(E::UserEnvCall) => syscall(tf),
        Trap::Exception(E::LoadPageFault) => page_fault(tf),
        Trap::Exception(E::StorePageFault) => page_fault(tf),
        Trap::Exception(E::InstructionPageFault) => page_fault(tf),
        _ => crate::trap::error(tf),
    }
    trace!("Interrupt end");
}

fn external() {
    #[cfg(feature = "board_u540")]
    unsafe { super::board::handle_external_interrupt(); }

    // true means handled, false otherwise
    let handlers = [try_process_serial, try_process_drivers];
    for handler in handlers.iter() {
        if handler() == true {
            break
        }
    }
}

fn try_process_serial() -> bool {
    match super::io::getchar_option() {
        Some(ch) => {
            crate::trap::serial(ch);
            true
        }
        None => false
    }
}

fn try_process_drivers() -> bool {
    for driver in DRIVERS.read().iter() {
        if driver.try_handle_interrupt(None) == true {
            return true
        }
    }
    return false
}

fn ipi() {
    debug!("IPI");
    super::sbi::clear_ipi();
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
    let ret = crate::syscall::syscall(tf.x[17], [tf.x[10], tf.x[11], tf.x[12], tf.x[13], tf.x[14], tf.x[15]], tf);
    tf.x[10] = ret as usize;
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

    if !crate::memory::handle_page_fault(addr) {
        crate::trap::error(tf);
    }
}
