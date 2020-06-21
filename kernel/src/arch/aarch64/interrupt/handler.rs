//! Trap handler

use super::syndrome::{Fault, Syndrome};
use crate::arch::board::irq::{handle_irq, is_timer_irq};
use trapframe::TrapFrame;

use aarch64::regs::*;
use log::*;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

impl Kind {
    fn from(x: usize) -> Kind {
        match x {
            x if x == Kind::Synchronous as usize => Kind::Synchronous,
            x if x == Kind::Irq as usize => Kind::Irq,
            x if x == Kind::Fiq as usize => Kind::Fiq,
            x if x == Kind::SError as usize => Kind::SError,
            _ => panic!("bad kind"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

impl Source {
    fn from(x: usize) -> Source {
        match x {
            x if x == Source::CurrentSpEl0 as usize => Source::CurrentSpEl0,
            x if x == Source::CurrentSpElx as usize => Source::CurrentSpElx,
            x if x == Source::LowerAArch64 as usize => Source::LowerAArch64,
            x if x == Source::LowerAArch32 as usize => Source::LowerAArch32,
            _ => panic!("bad kind"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Info {
    source: Source,
    kind: Kind,
}

/// This function is called when an exception occurs. The `info` parameter
/// specifies the source and kind of exception that has occurred. The `esr` is
/// the value of the exception syndrome register. Finally, `tf` is a pointer to
/// the trap frame for the exception.
#[no_mangle]
pub extern "C" fn trap_handler(tf: &mut TrapFrame) {
    let info: Info = Info {
        source: Source::from(tf.trap_num & 0xFFFF),
        kind: Kind::from(tf.trap_num >> 16),
    };
    let esr = ESR_EL1.get() as u32;
    info!(
        "Exception @ CPU{}: {:?}, ESR: {:#x}, ELR: {:#x?}",
        crate::arch::cpu::id(),
        info,
        esr,
        tf.elr
    );
    match info.kind {
        Kind::Synchronous => {
            let syndrome = Syndrome::from(esr);
            trace!("ESR: {:#x?}, Syndrome: {:?}", esr, syndrome);
            // syndrome is only valid with sync
            match syndrome {
                Syndrome::Brk(brk) => handle_break(brk, tf),
                Syndrome::Svc(svc) => handle_syscall(svc, tf),
                Syndrome::DataAbort { kind, level: _ }
                | Syndrome::InstructionAbort { kind, level: _ } => match kind {
                    Fault::Translation | Fault::AccessFlag | Fault::Permission => {
                        handle_page_fault(tf)
                    }
                    _ => panic!(), // crate::trap::error(tf),
                },
                _ => panic!(), //crate::trap::error(tf),
            }
        }
        Kind::Irq => {
            if is_timer_irq() {
                handle_timer()
            } else {
                handle_irq(tf)
            }
        }
        _ => panic!(),
    }
    trace!("Exception end");
}

fn handle_break(_num: u16, tf: &mut TrapFrame) {
    // Skip the current brk instruction (ref: J1.1.2, page 6147)
    tf.elr += 4;
}

fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    if num != 0 {
        panic!()
        //crate::trap::error(tf);
    }

    syscall(tf)
}

pub fn syscall(tf: &mut TrapFrame) {
    // svc instruction has been skipped in syscall (ref: J1.1.2, page 6152)
    /*
    let ret = crate::syscall::syscall(
        tf.x1to29[7] as usize,
        [
            tf.x0,
            tf.x1to29[0],
            tf.x1to29[1],
            tf.x1to29[2],
            tf.x1to29[3],
            tf.x1to29[4],
        ],
        tf,
    );
    tf.x0 = ret as usize;
    */
}

fn handle_timer() {
    crate::arch::board::timer::set_next();
    crate::trap::timer();
}

fn handle_page_fault(tf: &mut TrapFrame) {
    let addr = FAR_EL1.get() as usize;
    if !crate::memory::handle_page_fault(addr) {
        panic!("\nEXCEPTION: Page Fault @ {:#x}", addr);
    }
}
