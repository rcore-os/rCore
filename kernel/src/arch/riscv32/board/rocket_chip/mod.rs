use super::consts::KERNEL_OFFSET;

/// Mask all external interrupt except serial.
pub unsafe fn init_external_interrupt() {
    const HART0_S_MODE_INTERRUPT_ENABLES: *mut u64 = (KERNEL_OFFSET + 0x0C00_2080) as *mut u64;
    HART0_S_MODE_INTERRUPT_ENABLES.write_volatile(0xf);
}

/// Claim and complete external interrupt by reading and writing to
/// PLIC Interrupt Claim/Complete Register.
pub unsafe fn handle_external_interrupt() {
    const HART0_S_MODE_INTERRUPT_CLAIM_COMPLETE: *mut u32 =
        (KERNEL_OFFSET + 0x0C20_2000) as *mut u32;
    // claim
    let source = HART0_S_MODE_INTERRUPT_CLAIM_COMPLETE.read_volatile();
    // complete
    HART0_S_MODE_INTERRUPT_CLAIM_COMPLETE.write_volatile(source);
}