use crate::memory::phys_to_virt;

/// Mask all external interrupt except serial.
pub unsafe fn init_external_interrupt() {
    // By default:
    // riscv-pk (bbl) enables all S-Mode IRQs (ref: machine/minit.c)
    // OpenSBI v0.3 disables all IRQs (ref: platform/common/irqchip/plic.c)

    const HART0_S_MODE_INTERRUPT_ENABLES: *mut u32 = phys_to_virt(0x0C00_2080) as *mut u32;
    const SERIAL: u32 = 0xa;
    HART0_S_MODE_INTERRUPT_ENABLES.write_volatile(1 << SERIAL);
}

pub unsafe fn enable_serial_interrupt() {
    const UART16550: *mut u8 = phys_to_virt(0x10000000) as *mut u8;
    UART16550.add(4).write_volatile(0x0B);
    UART16550.add(1).write_volatile(0x01);
}
