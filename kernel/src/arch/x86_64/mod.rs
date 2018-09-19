extern crate bootloader;

use self::bootloader::bootinfo::{BootInfo, MemoryRegionType};

pub mod driver;
pub mod cpu;
pub mod interrupt;
pub mod paging;
pub mod gdt;
pub mod idt;
// TODO: Move multi-core init to bootloader
//pub mod smp;
pub mod memory;
pub mod io;

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    // First init log mod, so that we can print log info.
    ::logging::init();
    info!("Hello world!");
    info!("{:#?}", boot_info);

    // Init trap handling.
    idt::init();

    // Init physical memory management and heap.
    memory::init(boot_info);

    // Now heap is available
    gdt::init();

    driver::init();

    ::kmain();
}

/// The entry point for another processors
#[no_mangle]
pub extern "C" fn other_main() -> ! {
    idt::init();
    gdt::init();
    driver::apic::other_init();
    let cpu_id = driver::apic::lapic_id();
//    let ms = unsafe { smp::notify_started(cpu_id) };
    println!("Hello world! from CPU {}!", cpu_id);
//    unsafe{ let a = *(0xdeadbeaf as *const u8); } // Page fault
    loop {}
}