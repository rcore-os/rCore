pub mod vga;
pub mod acpi;
pub mod apic;
pub mod mp;
pub mod serial;
pub mod pic;
pub mod keyboard;
pub mod pit;
pub mod ide;

pub fn init(mut page_map: impl FnMut(usize, usize)) -> acpi::AcpiResult {
    assert_has_not_been_called!();

    page_map(0, 1); // EBDA
    page_map(0xe0_000, 0x100 - 0xe0);
    page_map(0x07fe1000, 1); // RSDT
    page_map(0xfee00000, 1);  // LAPIC
    page_map(0xfec00000, 1);  // IOAPIC

    let acpi = acpi::init().expect("Failed to init ACPI");
    assert_eq!(acpi.lapic_addr as usize, 0xfee00000);
    trace!("acpi = {:?}", acpi);

    if cfg!(feature = "use_apic") {
        pic::disable();
        apic::init(acpi.lapic_addr, acpi.ioapic_id);
    } else {
        pic::init();
    }
    pit::init();
    serial::init();
    keyboard::init();
    acpi
}