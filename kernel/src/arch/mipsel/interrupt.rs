use mips::interrupts;
use mips::tlb;
use mips::registers::cp0;
use crate::drivers::DRIVERS;
use mips::paging::{PageTable as MIPSPageTable, PageTableEntry, PageTableFlags as EF, TwoLevelPageTable};
use mips::addr::*;
pub use self::context::*;
use crate::arch::paging::get_root_page_table_ptr;
use log::*;

#[path = "context.rs"]
mod context;

/// Initialize interrupt
pub fn init() {
    extern {
        fn trap_entry();
    }
    unsafe {
        // Set the exception vector address
        cp0::ebase::write_u32(trap_entry as u32);
        println!("Set ebase = {:x}", trap_entry as u32);

        let mut status = cp0::status::read();
        // Enable IPI
        status.enable_soft_int0();
        status.enable_soft_int1();
        // Enable clock interrupt
        status.enable_hard_int5();

        cp0::status::write(status);
    }
    info!("interrupt: init end");
}

/// Enable interrupt
#[inline]
pub unsafe fn enable() {
    interrupts::enable();
}

/// Disable interrupt and return current interrupt status
#[inline]
pub unsafe fn disable_and_store() -> usize {
    let e = cp0::status::read_u32() & 1;
    interrupts::disable();
    e as usize
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
pub extern fn rust_trap(tf: &mut TrapFrame) {
    use cp0::cause::{Exception as E};
    trace!("Interrupt @ CPU{}: {:?} ", 0, tf.cause.cause());
    match tf.cause.cause() {
        E::Interrupt => interrupt_dispatcher(tf),
        E::Syscall => syscall(tf),
        E::TLBModification => page_fault(tf),
        E::TLBLoadMiss => page_fault(tf),
        E::TLBStoreMiss => page_fault(tf),
        _ => crate::trap::error(tf),
    }
    trace!("Interrupt end");
}

fn interrupt_dispatcher(tf: &mut TrapFrame) {
    let pint = tf.cause.pending_interrupt();
    if (pint & 0b10000_00) != 0 {
        timer();
    } else if (pint & 0xb01111_00) != 0 {
        external();
    } else {
        ipi();
    }
}

fn external() {
    // TODO
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
    // TODO
    for driver in DRIVERS.read().iter() {
        if driver.try_handle_interrupt(None) == true {
            return true
        }
    }
    return false
}

fn ipi() {
    /* do nothing */
    debug!("IPI");
//    super::sbi::clear_ipi();
}

fn timer() {
    super::timer::set_next();
    crate::trap::timer();
}

fn syscall(tf: &mut TrapFrame) {
    tf.epc += 4;   // Must before syscall, because of fork.
    let ret = crate::syscall::syscall(tf.t0, [tf.t0, tf.t1, tf.t2, tf.t3, tf.s0, tf.s1], tf);
    tf.v0 = ret as usize;
}

fn page_fault(tf: &mut TrapFrame) {
    // TODO: set access/dirty bit
    let addr = tf.vaddr;
    trace!("\nEXCEPTION: Page Fault @ {:#x}", addr);

    let virt_addr = VirtAddr::new(addr);
    let root_table = unsafe {
        &mut *(get_root_page_table_ptr() as *mut MIPSPageTable)
    };
    let tlb_result = root_table.lookup(addr);
    match tlb_result {
        Ok(tlb_entry) => tlb::write_tlb_random(tlb_entry),
        Err(()) => if !crate::memory::handle_page_fault(addr) {
            crate::trap::error(tf);
        }
    }
}
