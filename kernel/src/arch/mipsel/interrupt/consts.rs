use mips::registers::cp0;

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

pub fn is_syscall(trap: usize) -> bool {
    use cp0::cause::Exception as E;
    let cause = cp0::cause::Cause { bits: trap as u32 };
    match cause.cause() {
        E::Syscall => true,
        _ => false,
    }
}

pub fn is_intr(trap: usize) -> bool {
    use cp0::cause::Exception as E;
    let cause = cp0::cause::Cause { bits: trap as u32 };
    match cause.cause() {
        E::Interrupt => true,
        _ => false,
    }
}

pub fn is_timer_intr(trap: usize) -> bool {
    use cp0::cause::Exception as E;
    let cause = cp0::cause::Cause { bits: trap as u32 };
    match cause.cause() {
        E::Interrupt => trap & (1 << 30) != 0,
        _ => false,
    }
}
