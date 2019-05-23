use super::consts::KERNEL_OFFSET;
use crate::memory::phys_to_virt;

/// Mask all external interrupt except serial.
pub unsafe fn init_external_interrupt() {
    const HART1_S_MODE_INTERRUPT_ENABLES: *mut u64 = phys_to_virt(0x0C00_2100) as *mut u64;
    const SERIAL: u64 = 4;
    HART1_S_MODE_INTERRUPT_ENABLES.write_volatile(1 << SERIAL);
}

/// Claim and complete external interrupt by reading and writing to
/// PLIC Interrupt Claim/Complete Register.
pub unsafe fn handle_external_interrupt() {
    const HART1_S_MODE_INTERRUPT_CLAIM_COMPLETE: *mut u32 = phys_to_virt(0x0C20_2004) as *mut u32;
    // claim
    let source = HART1_S_MODE_INTERRUPT_CLAIM_COMPLETE.read_volatile();
    // complete
    HART1_S_MODE_INTERRUPT_CLAIM_COMPLETE.write_volatile(source);
}

pub unsafe fn enable_serial_interrupt() {
    const SERIAL_BASE: *mut u8 = phys_to_virt(0x10010000) as *mut u8;
    const UART_REG_IE: usize = 4;
    const UART_RXWM: u8 = 0x2;
    SERIAL_BASE.add(UART_REG_IE).write_volatile(UART_RXWM);
}
