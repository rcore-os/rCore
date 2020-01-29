use crate::memory::phys_to_virt;

/// Device tree bytes
pub static DTB: &'static [u8] = include_bytes!("device.dtb");

/// Mask all external interrupt except serial.
pub unsafe fn init_external_interrupt() {
    const HART0_S_MODE_INTERRUPT_ENABLES: *mut u64 = phys_to_virt(0x0C00_2080) as *mut u64;
    // enable all external interrupts
    HART0_S_MODE_INTERRUPT_ENABLES.write_volatile(0xf);

    // mask interrupts first
    const AXI_INTC_IER: *mut u32 = phys_to_virt(0x6120_0008) as *mut u32;
    AXI_INTC_IER.write_volatile(0x0);

    // acknowledge all interrupts
    const AXI_INTC_IAR: *mut u32 = phys_to_virt(0x6120_000C) as *mut u32;
    AXI_INTC_IAR.write_volatile(0xffffffff);

    const AXI_INTC_MER: *mut u32 = phys_to_virt(0x6120_001C) as *mut u32;
    // Hardware Interrupt enable | Enable irq output
    AXI_INTC_MER.write_volatile(0b11);

    // enable all interrupts
    AXI_INTC_IER.write_volatile(0xffffffff);
}

/// Claim and complete external interrupt by reading and writing to
/// PLIC Interrupt Claim/Complete Register.
pub unsafe fn handle_external_interrupt() {
    const HART0_S_MODE_INTERRUPT_CLAIM_COMPLETE: *mut u32 = phys_to_virt(0x0C20_1004) as *mut u32;
    // claim
    let source = HART0_S_MODE_INTERRUPT_CLAIM_COMPLETE.read_volatile();
    // complete
    HART0_S_MODE_INTERRUPT_CLAIM_COMPLETE.write_volatile(source);

    // acknowledge all interrupts
    const AXI_INTC_IAR: *mut u32 = phys_to_virt(0x6120_000C) as *mut u32;
    AXI_INTC_IAR.write_volatile(0xffffffff);
}

pub unsafe fn enable_serial_interrupt() {
    const SERIAL_BASE: *mut u32 = phys_to_virt(0x60000000) as *mut u32;
    const UART_CTRL_REG: usize = 3;
    // Intr enable | rx reset | tx reset
    const UART_IE: u32 = 0x13;
    SERIAL_BASE.add(UART_CTRL_REG).write_volatile(UART_IE);
}
