extern crate multiboot2;

use memory::MemorySet;

pub mod driver;
pub mod cpu;
pub mod interrupt;
pub mod paging;
pub mod gdt;
pub mod idt;
pub mod smp;
pub mod memory;

pub fn init() {
    idt::init();
    // Load boot info address from stack_top.
    // See `boot.asm`
    extern {
        fn stack_top();
    }
    let boot_info_addr = unsafe { *(stack_top as *const u32).offset(-1) } as usize;
    let boot_info = unsafe { multiboot2::load(boot_info_addr) };
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