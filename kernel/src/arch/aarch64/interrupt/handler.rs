//! Trap handler

use arch::board::irq::handle_irq;
use super::context::TrapFrame;
use super::syndrome::Syndrome;

global_asm!(include_str!("trap.S"));
global_asm!(include_str!("vector.S"));

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Kind {
    Synchronous = 0,
    Irq = 1,
    Fiq = 2,
    SError = 3,
}

#[repr(u16)]
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Source {
    CurrentSpEl0 = 0,
    CurrentSpElx = 1,
    LowerAArch64 = 2,
    LowerAArch32 = 3,
}

#[repr(C)]
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
pub extern "C" fn rust_trap(info: Info, esr: u32, tf: &mut TrapFrame) {
    let syndrome = Syndrome::from(esr);
    trace!("Interrupt: {:?} from: {:?}", syndrome, info);
    match info.kind {
        Kind::Synchronous => {
            // syndrome is only valid with sync
            match syndrome {
                Syndrome::Brk(brk) => handle_break(brk, tf),
                Syndrome::Svc(syscall) => handle_syscall(syscall, tf),
                _ => ::trap::error(tf),
            }
        }
        Kind::Irq => handle_irq(),
        _ => ::trap::error(tf),
    }
    ::trap::before_return();
    trace!("Interrupt end");
}

fn handle_break(num: u16, tf: &mut TrapFrame) {
    tf.elr += 4; // Skip the current brk instruction
}

fn handle_syscall(num: u16, tf: &mut TrapFrame) {
    // svc instruction has been skipped in syscall
    println!("syscall {}", num);
}
