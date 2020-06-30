//! Trap handler

use super::syndrome::{Fault, Syndrome};
use crate::arch::board::timer;
use crate::drivers::IRQ_MANAGER;
use aarch64::regs::*;
use log::*;
use trapframe::TrapFrame;

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
    trace!(
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
                Syndrome::DataAbort { kind, level: _ }
                | Syndrome::InstructionAbort { kind, level: _ } => match kind {
                    Fault::Translation | Fault::AccessFlag | Fault::Permission => {
                        let addr = FAR_EL1.get() as usize;
                        if !crate::memory::handle_page_fault(addr) {
                            panic!("\nEXCEPTION: Page Fault @ {:#x}", addr);
                        }
                    }
                    _ => panic!(),
                },
                _ => panic!(),
            }
        }
        Kind::Irq => {
            if timer::is_pending() {
                crate::arch::board::timer::set_next();
                crate::trap::timer();
            } else {
                IRQ_MANAGER.read().try_handle_interrupt(Some(tf.trap_num));
            }
        }
        _ => panic!(),
    }
    trace!("Exception end");
}
