pub use self::context::*;
use crate::arch::paging::get_root_page_table_ptr;
use crate::drivers::{DRIVERS, IRQ_MANAGER};
use log::*;
use mips::addr::*;
use mips::interrupts;
use mips::paging::{
    PageTable as MIPSPageTable, PageTableEntry, PageTableFlags as EF, TwoLevelPageTable,
};
use mips::registers::cp0;
use mips::tlb;

#[path = "context.rs"]
mod context;

/// Initialize interrupt
pub fn init() {
    extern "C" {
        fn trap_entry();
    }
    // Set the exception vector address
    cp0::ebase::write_u32(trap_entry as u32);
    println!("Set ebase = {:x}", trap_entry as u32);

    let mut status = cp0::status::read();
    // Enable IPI
    status.enable_soft_int0();
    status.enable_soft_int1();
    // Enable clock interrupt
    status.enable_hard_int5();
    // Enable serial interrupt
    #[cfg(feature = "board_thinpad")]
    status.enable_hard_int0();

    cp0::status::write(status);
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

#[no_mangle]
pub extern "C" fn stack_pointer_not_aligned(sp: usize) {
    panic!("Stack pointer not aligned: sp = 0x{:x?}", sp);
}

/// Dispatch and handle interrupt.
///
/// This function is called from `trap.asm`.
#[no_mangle]
pub extern "C" fn rust_trap(tf: &mut TrapFrame) {
    use cp0::cause::Exception as E;
    trace!("Exception @ CPU{}: {:?} ", 0, tf.cause.cause());
    match tf.cause.cause() {
        E::Interrupt => interrupt_dispatcher(tf),
        E::Syscall => syscall(tf),
        E::TLBModification => page_fault(tf),
        E::TLBLoadMiss => page_fault(tf),
        E::TLBStoreMiss => page_fault(tf),
        E::ReservedInstruction => {
            if !reserved_inst(tf) {
                error!("Unhandled Exception @ CPU{}: {:?} ", 0, tf.cause.cause());
                crate::trap::error(tf)
            } else {
                tf.epc = tf.epc + 4;
            }
        }
        E::CoprocessorUnusable => {
            tf.epc = tf.epc + 4;
        }
        _ => {
            error!("Unhandled Exception @ CPU{}: {:?} ", 0, tf.cause.cause());
            crate::trap::error(tf)
        }
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
            break;
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
        None => false,
    }
}

fn try_process_drivers() -> bool {
    IRQ_MANAGER.read().try_handle_interrupt(None)
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
    tf.epc += 4; // Must before syscall, because of fork.
    let arguments = [tf.a0, tf.a1, tf.a2, tf.a3, tf.t0, tf.t1];
    trace!(
        "MIPS syscall {} invoked with {:x?}, epc = {:x?}",
        tf.v0,
        arguments,
        tf.epc
    );

    // temporary solution for ThinPad
    if (tf.v0 == 0) {
        warn!("Syscall ID = 0");
        tf.v0 = unsafe { *((tf.sp + 28) as *const usize) };
    }

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

fn set_trapframe_register(rt: usize, val: usize, tf: &mut TrapFrame) {
    match rt {
        1 => tf.at = val,
        2 => tf.v0 = val,
        3 => tf.v1 = val,
        4 => tf.a0 = val,
        5 => tf.a1 = val,
        6 => tf.a2 = val,
        7 => tf.a3 = val,
        8 => tf.t0 = val,
        9 => tf.t1 = val,
        10 => tf.t2 = val,
        11 => tf.t3 = val,
        12 => tf.t4 = val,
        13 => tf.t5 = val,
        14 => tf.t6 = val,
        15 => tf.t7 = val,
        16 => tf.s0 = val,
        17 => tf.s1 = val,
        18 => tf.s2 = val,
        19 => tf.s3 = val,
        20 => tf.s4 = val,
        21 => tf.s5 = val,
        22 => tf.s6 = val,
        23 => tf.s7 = val,
        24 => tf.t8 = val,
        25 => tf.t9 = val,
        26 => tf.k0 = val,
        27 => tf.k1 = val,
        28 => tf.gp = val,
        29 => tf.sp = val,
        30 => tf.fp = val,
        31 => tf.ra = val,
        _ => {
            error!("Unknown register {:?} ", rt);
            crate::trap::error(tf)
        }
    }
}

fn reserved_inst(tf: &mut TrapFrame) -> bool {
    let inst = unsafe { *(tf.epc as *const usize) };

    let opcode = inst >> 26;
    let rt = (inst >> 16) & 0b11111;
    let rd = (inst >> 11) & 0b11111;
    let sel = (inst >> 6) & 0b111;
    let format = inst & 0b111111;

    if inst == 0x42000020 {
        // ignore WAIT
        return true;
    }

    if opcode == 0b011111 && format == 0b111011 {
        // RDHWR
        if rd == 29 && sel == 0 {
            extern "C" {
                fn _cur_tls();
            }

            let tls = unsafe { *(_cur_tls as *const usize) };

            set_trapframe_register(rt, tls, tf);
            debug!("Read TLS by rdhdr {:x} to register {:?}", tls, rt);
            return true;
        } else {
            return false;
        }
    }

    false
}

fn page_fault(tf: &mut TrapFrame) {
    // TODO: set access/dirty bit
    let addr = tf.vaddr;
    trace!("\nEXCEPTION: Page Fault @ {:#x}", addr);

    let virt_addr = VirtAddr::new(addr);
    let root_table = unsafe { &mut *(get_root_page_table_ptr() as *mut MIPSPageTable) };
    let tlb_result = root_table.lookup(addr);
    match tlb_result {
        Ok(tlb_entry) => {
            trace!(
                "PhysAddr = {:x}/{:x}",
                tlb_entry.entry_lo0.get_pfn() << 12,
                tlb_entry.entry_lo1.get_pfn() << 12
            );

            let tlb_valid = if virt_addr.page_number() & 1 == 0 {
                tlb_entry.entry_lo0.valid()
            } else {
                tlb_entry.entry_lo1.valid()
            };

            if !tlb_valid {
                if !crate::memory::handle_page_fault(addr) {
                    extern "C" {
                        fn _copy_user_start();
                        fn _copy_user_end();
                    }
                    if tf.epc >= _copy_user_start as usize && tf.epc < _copy_user_end as usize {
                        debug!("fixup for addr {:x?}", addr);
                        tf.epc = crate::memory::read_user_fixup as usize;
                        return;
                    }
                    crate::trap::error(tf);
                }
            }

            tlb::write_tlb_random(tlb_entry)
        }
        Err(()) => {
            if !crate::memory::handle_page_fault(addr) {
                extern "C" {
                    fn _copy_user_start();
                    fn _copy_user_end();
                }
                if tf.epc >= _copy_user_start as usize && tf.epc < _copy_user_end as usize {
                    debug!("fixup for addr {:x?}", addr);
                    tf.epc = crate::memory::read_user_fixup as usize;
                    return;
                }
                crate::trap::error(tf);
            }
        }
    }
}
