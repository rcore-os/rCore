pub mod vga;
pub mod acpi;
pub mod apic;
pub mod mp;
pub mod serial;
pub mod pic;
pub mod keyboard;
pub mod pit;
pub mod ide;

pub fn init(rsdt_addr: usize) -> acpi::AcpiResult {
    assert_has_not_been_called!();

    let acpi = acpi::init(rsdt_addr).expect("Failed to init ACPI");
    assert_eq!(acpi.lapic_addr as usize, 0xfee00000);
    trace!("acpi = {:?}", acpi);

    if cfg!(feature = "use_apic") {
        pic::disable();
        use consts::KERNEL_OFFSET;
        apic::init((KERNEL_OFFSET + 0xfee00000) as *const (), acpi.ioapic_id);
    } else {
        pic::init();
    }
    pit::init();
    serial::init();
    keyboard::init();
    acpi
}