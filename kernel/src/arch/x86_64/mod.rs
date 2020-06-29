use core::sync::atomic::*;
use log::*;
use rboot::BootInfo;

pub mod acpi;
pub mod board;
pub mod consts;
pub mod cpu;
pub mod gdt;
pub mod interrupt;
pub mod io;
pub mod ipi;
pub mod memory;
pub mod paging;
pub mod rand;
pub mod signal;
pub mod syscall;
pub mod timer;

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn _start(boot_info: &'static BootInfo) -> ! {
    let cpu_id = cpu::id();

    if cpu_id != 0 {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {
            spin_loop_hint();
        }
        other_start();
    }

    // init log and heap
    crate::logging::init();
    crate::memory::init_heap();

    // serial
    board::early_init();

    println!("Hello world! from CPU {}!", cpu_id);

    // check BootInfo from bootloader
    info!("{:#x?}", boot_info);
    assert_eq!(
        boot_info.physical_memory_offset as usize,
        consts::PHYSICAL_MEMORY_OFFSET
    );

    // Init physical memory management
    memory::init(boot_info);

    // Init trap handler
    unsafe {
        trapframe::init();
    }

    // init virtual space
    memory::init_kernel_kseg2_map();
    // init local apic
    cpu::init();
    // now we can start LKM.
    crate::lkm::manager::ModuleManager::init();
    // init board
    board::init(boot_info);
    // init cpu scheduler and process manager, and add user shell app in process manager
    crate::process::init();
    // load acpi
    acpi::init(boot_info.acpi2_rsdp_addr as usize);

    // wake up other CPUs
    AP_CAN_INIT.store(true, Ordering::Relaxed);

    // call the first main function in kernel.
    crate::kmain();
}

/// The entry point for other processors
fn other_start() -> ! {
    // init trap handling
    unsafe {
        trapframe::init();
    }
    // init local apic
    cpu::init();
    // call the first main function in kernel.
    crate::kmain();
}
