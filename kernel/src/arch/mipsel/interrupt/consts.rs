use mips::registers::cp0;

pub const IrqMin: usize = 0x20;
pub const IrqMax: usize = 0x3f;
pub const Syscall: usize = 0x100;

pub const Timer: usize = IrqMin + 0;

pub fn is_page_fault(trap: usize) -> bool {
    use cp0::cause::Exception as E;
    let cause = cp0::cause::Cause { bits: trap as u32 };
    match cause.cause() {
        E::TLBModification => true,
        E::TLBLoadMiss => true,
        E::TLBStoreMiss => true,
        _ => false,
    }
}
