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
    trace!("Exception @ CPU{}: {:?} ", 0, tf.cause.cause());
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
    trace!("  Interrupt {:08b} ", pint);
    if (pint & 0b100_000_00) != 0 {
        timer();
    } else if (pint & 0b011_111_00) != 0 {
        external();
    } else {
        ipi();
    }
}

fn external() {
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
            trace!("Get char {} from serial", ch);
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
    debug!("IPI");
    cp0::cause::reset_soft_int0();
    cp0::cause::reset_soft_int1();
}

fn timer() {
    super::timer::set_next();
    crate::trap::timer();
}

fn syscall(tf: &mut TrapFrame) {
    tf.epc += 4;   // Must before syscall, because of fork.
    let arguments = [tf.a0, tf.a1, tf.a2, tf.a3, tf.t0, tf.t1];
    trace!("MIPS syscall {} invoked with {:?}", tf.v0, arguments);

    let ret = crate::syscall::syscall(tf.v0, arguments, tf) as isize;
    // comply with mips n32 abi, always return a positive value
    // https://git.musl-libc.org/cgit/musl/tree/arch/mipsn32/syscall_arch.h
    if (ret < 0) {
        tf.v0 = (-ret) as usize;
        tf.a3 = 1;
    } else {
        tf.v0 = ret as usize;
        tf.a3 = 0;
    }
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
        Ok(tlb_entry) => {
            trace!("PhysAddr = {:x}/{:x}",
                   tlb_entry.entry_lo0.get_pfn() << 12,
                   tlb_entry.entry_lo1.get_pfn() << 12);
            tlb::write_tlb_random(tlb_entry)
        },
        Err(()) => {
            if !crate::memory::handle_page_fault(addr) {
                crate::trap::error(tf);
            }
        }
    }
}
