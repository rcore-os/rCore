extern crate bootloader;
extern crate apic;
extern crate raw_cpuid;

use self::bootloader::bootinfo::{BootInfo, MemoryRegionType};
use core::sync::atomic::*;
use consts::KERNEL_OFFSET;

pub mod driver;
pub mod cpu;
pub mod interrupt;
pub mod paging;
pub mod gdt;
pub mod idt;
pub mod memory;
pub mod io;
pub mod consts;

static AP_CAN_INIT: AtomicBool = ATOMIC_BOOL_INIT;

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    let cpu_id = cpu::id();
    println!("Hello world! from CPU {}!", cpu_id);

    if cpu_id != 0 {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {}
        other_start();
    }

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

    cpu::init();

    driver::init();

    AP_CAN_INIT.store(true, Ordering::Relaxed);

    ::kmain();
}

/// The entry point for other processors
fn other_start() -> ! {
    idt::init();
    gdt::init();
    cpu::init();
    ::kmain();
}