use super::syndrome::{Fault, Syndrome};

pub fn is_page_fault(trap: usize) -> bool {
    let syndrome = Syndrome::from(trap as u32);
    match syndrome {
        Syndrome::DataAbort { kind, level: _ } | Syndrome::InstructionAbort { kind, level: _ } => {
            match kind {
                Fault::Translation | Fault::AccessFlag | Fault::Permission => true,
                _ => false,
            }
        }
        _ => false,
    }
}

// from el0, irq
pub const IrqMax: usize = 0x10002_00000000;
pub const IrqMin: usize = 0x10002_00000000;
pub const Timer: usize = 0x10002_00000000;

pub const Syscall: usize = 0b010001_000000;
