use crate::arch::paging::get_root_page_table_ptr;
use crate::drivers::IRQ_MANAGER;
use crate::process::thread::Thread;
use alloc::sync::Arc;
use log::*;
use mips::addr::*;
use mips::interrupts;
use mips::paging::PageTable as MIPSPageTable;
use mips::registers::cp0;
use trapframe::{TrapFrame, UserContext};

pub mod consts;

/// Initialize interrupt
pub fn init() {
    unsafe {
        trapframe::init();
    }

    let mut status = cp0::status::read();
    // Enable IPI
    status.enable_soft_int0();
    status.enable_soft_int1();
    // Enable clock interrupt
    status.enable_hard_int5();

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
pub extern "C" fn trap_handler(tf: &mut TrapFrame) {
    use cp0::cause::Exception as E;
    let cause = cp0::cause::Cause {
        bits: tf.cause as u32,
    };
    debug!("Exception @ CPU{}: {:?} ", 0, cause.cause());
    match cause.cause() {
        E::Interrupt => interrupt_dispatcher(tf),
        E::Syscall => syscall(tf),
        E::TLBModification => page_fault(tf),
        E::TLBLoadMiss => page_fault(tf),
        E::TLBStoreMiss => page_fault(tf),
        E::ReservedInstruction => {
            if !reserved_inst(tf) {
                error!("Unhandled Exception @ CPU{}: {:?} ", 0, cause.cause());
            } else {
                tf.epc = tf.epc + 4;
            }
        }
        E::CoprocessorUnusable => {
            tf.epc = tf.epc + 4;
        }
        _ => {
            error!("Unhandled Exception @ CPU{}: {:?} ", 0, cause.cause());
        }
    }
    trace!("Interrupt end");
}

fn interrupt_dispatcher(tf: &mut TrapFrame) {
    /*
    let pint = tf.cause.pending_interrupt();
    trace!("  Interrupt {:08b} ", pint);
    if (pint & 0b100_000_00) != 0 {
        timer();
    } else if (pint & 0b011_111_00) != 0 {
        external();
    } else {
        ipi();
    }
    */
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

pub fn timer() {
    super::timer::set_next();
    crate::trap::timer();
}

fn syscall(tf: &mut TrapFrame) {
    tf.epc += 4; // Must before syscall, because of fork.
                 /*
                 let arguments = [tf.a0, tf.a1, tf.a2, tf.a3, tf.t0, tf.t1];
                 trace!(
                     "MIPS syscall {} invoked with {:x?}, epc = {:x?}",
                     tf.v0,
                     arguments,
                     tf.epc
                 );
                 */

    //let ret = crate::syscall::syscall(tf.v0, arguments, tf) as isize;
    let ret = 0 as isize;
    // comply with mips n32 abi, always return a positive value
    // https://git.musl-libc.org/cgit/musl/tree/arch/mipsn32/syscall_arch.h
    if ret < 0 {
        tf.general.v0 = (-ret) as usize;
        tf.general.a3 = 1;
    } else {
        tf.general.v0 = ret as usize;
        tf.general.a3 = 0;
    }
}

fn set_trapframe_register(rt: usize, val: usize, tf: &mut TrapFrame) {
    match rt {
        1 => tf.general.at = val,
        2 => tf.general.v0 = val,
        3 => tf.general.v1 = val,
        4 => tf.general.a0 = val,
        5 => tf.general.a1 = val,
        6 => tf.general.a2 = val,
        7 => tf.general.a3 = val,
        8 => tf.general.t0 = val,
        9 => tf.general.t1 = val,
        10 => tf.general.t2 = val,
        11 => tf.general.t3 = val,
        12 => tf.general.t4 = val,
        13 => tf.general.t5 = val,
        14 => tf.general.t6 = val,
        15 => tf.general.t7 = val,
        16 => tf.general.s0 = val,
        17 => tf.general.s1 = val,
        18 => tf.general.s2 = val,
        19 => tf.general.s3 = val,
        20 => tf.general.s4 = val,
        21 => tf.general.s5 = val,
        22 => tf.general.s6 = val,
        23 => tf.general.s7 = val,
        24 => tf.general.t8 = val,
        25 => tf.general.t9 = val,
        26 => tf.general.k0 = val,
        27 => tf.general.k1 = val,
        28 => tf.general.gp = val,
        29 => tf.general.sp = val,
        30 => tf.general.fp = val,
        31 => tf.general.ra = val,
        _ => {
            error!("Unknown register {:?} ", rt);
            //crate::trap::error(tf)
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

pub fn handle_user_page_fault(thread: &Arc<Thread>, addr: usize) -> bool {
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
                if !thread.vm.lock().handle_page_fault(addr) {
                    return false;
                }
            }

            tlb_entry.write_random();
            true
        }
        Err(()) => {
            return thread.vm.lock().handle_page_fault(addr);
        }
    }
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
                    //crate::trap::error(tf);
                }
            }

            tlb_entry.write_random()
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
                //crate::trap::error(tf);
            }
        }
    }
}

pub fn enable_irq(irq: usize) {
    // TODO
}

pub fn get_trap_num(cx: &UserContext) -> usize {
    cx.cause
}

pub fn ack(_irq: usize) {
    // TODO
}

pub fn wait_for_interrupt() {
    // TODO
}
