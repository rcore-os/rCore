use memory::MemorySet;
use multiboot2;

pub mod driver;
pub mod cpu;
pub mod interrupt;
pub mod paging;
pub mod gdt;
pub mod idt;
pub mod smp;
pub mod memory;

pub fn init(multiboot_information_address: usize) {
    idt::init();
    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };
    let rsdt_addr = boot_info.rsdp_v1_tag().unwrap().rsdt_address();
    memory::init(boot_info);
    // Now heap is available
    gdt::init();
    let acpi = driver::init(rsdt_addr);
    smp::start_other_cores(&acpi);
}

/// The entry point for another processors
#[no_mangle]
pub extern "C" fn other_main() -> ! {
    idt::init();
    gdt::init();
    driver::apic::other_init();
    let cpu_id = driver::apic::lapic_id();
    let ms = unsafe { smp::notify_started(cpu_id) };
    println!("Hello world! from CPU {}!", cpu_id);
//    unsafe{ let a = *(0xdeadbeaf as *const u8); } // Page fault
    loop {}
}