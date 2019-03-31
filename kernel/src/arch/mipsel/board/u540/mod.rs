use super::consts::KERNEL_OFFSET;

/// Mask all external interrupt except serial.
pub unsafe fn init_external_interrupt() {
    const HART1_S_MODE_INTERRUPT_ENABLES: *mut u64 = (KERNEL_OFFSET + 0x0C00_2100) as *mut u64;
    const SERIAL: u64 = 4;
    HART1_S_MODE_INTERRUPT_ENABLES.write(1 << SERIAL);
}

/// Claim and complete external interrupt by reading and writing to
/// PLIC Interrupt Claim/Complete Register.
pub unsafe fn handle_external_interrupt() {
    const HART1_S_MODE_INTERRUPT_CLAIM_COMPLETE: *mut u32 = (KERNEL_OFFSET + 0x0C20_2004) as *mut u32;
    // claim
    let source = HART1_S_MODE_INTERRUPT_CLAIM_COMPLETE.read();
    // complete
    HART1_S_MODE_INTERRUPT_CLAIM_COMPLETE.write(source);
}