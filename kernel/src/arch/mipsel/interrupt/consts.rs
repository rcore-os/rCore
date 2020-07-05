pub const IrqMin: usize = 0x20;
pub const IrqMax: usize = 0x3f;
pub const Syscall: usize = 0x100;

pub const Timer: usize = IrqMin + 0;

pub fn is_page_fault(trap: usize) -> bool {
    false
}
