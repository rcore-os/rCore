use crate::memory::phys_to_virt;
use riscv::register::sie;

/// Enable external interrupt
pub unsafe fn init_external_interrupt() {
    sie::set_sext();
}
