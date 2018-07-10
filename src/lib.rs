#![feature(ptr_internals)]
#![feature(lang_items)]
#![feature(const_fn)]
#![feature(alloc)]
#![feature(const_unique_new, const_atomic_usize_new)]
#![feature(unique)]
#![feature(allocator_api)]
#![feature(global_allocator)]
#![feature(abi_x86_interrupt)]
#![feature(iterator_step_by)]
#![feature(unboxed_closures)]
#![feature(naked_functions)]
#![feature(asm)]
#![feature(optin_builtin_traits)]
#![feature(panic_implementation)]
#![feature(panic_info_message)]
#![feature(universal_impl_trait)]
#![feature(global_asm)]
#![no_std]


#[macro_use]
extern crate alloc;
extern crate bit_allocator;
extern crate bit_field;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
extern crate linked_list_allocator;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate log;
#[cfg(target_arch = "x86_64")]
extern crate multiboot2;
#[macro_use]
extern crate once;
extern crate rlibc;
#[cfg(target_arch = "x86_64")]
extern crate simple_filesystem;
extern crate spin;
#[cfg(target_arch = "x86_64")]
extern crate syscall as redox_syscall;
#[cfg(target_arch = "x86_64")]
extern crate uart_16550;
extern crate ucore_memory;
extern crate volatile;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate x86_64;
extern crate xmas_elf;

pub use arch::interrupt::rust_trap;
#[cfg(target_arch = "x86_64")]
pub use arch::interrupt::set_return_rsp;
use linked_list_allocator::LockedHeap;

#[macro_use]    // print!
#[cfg(target_arch = "x86_64")]
mod io;

#[macro_use]    // print!
#[cfg(target_arch = "riscv")]
#[path = "io/riscv_io.rs"]
mod io;

#[cfg(target_arch = "x86_64")]
mod memory;
mod lang;
mod util;
mod consts;
#[cfg(target_arch = "x86_64")]
mod process;
#[cfg(target_arch = "x86_64")]
mod syscall;
#[cfg(target_arch = "x86_64")]
mod fs;
#[cfg(target_arch = "x86_64")]
mod thread;
#[cfg(target_arch = "x86_64")]
mod sync;

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
mod arch;

#[cfg(target_arch = "riscv")]
#[path = "arch/riscv32/mod.rs"]
mod arch;

#[no_mangle]
#[cfg(target_arch = "riscv")]
pub extern fn rust_main() -> ! {
    arch::init();
    loop {}
}

/// The entry point of Rust kernel
#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
    arch::idt::init();
    io::init();

    // ATTENTION: we have a very small stack and no guard page
    println!("Hello World{}", "!");

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };
    let rsdt_addr = boot_info.rsdp_v1_tag().unwrap().rsdt_address();

    // set up guard page and map the heap pages
    let mut kernel_memory = memory::init(boot_info);

    arch::gdt::init();

    memory::test::cow();

    let acpi = arch::driver::init(rsdt_addr, |addr: usize, count: usize| {
        use memory::*;
        kernel_memory.push(MemoryArea::new_identity(addr, addr + count * 0x1000, MemoryAttr::default(), "acpi"))
    });

    arch::smp::start_other_cores(&acpi, &mut kernel_memory);
    process::init(kernel_memory);

    fs::load_sfs();

    unsafe{ arch::interrupt::enable(); }

//    thread::test::unpack();
//    sync::test::philosopher_using_mutex();
//    sync::test::philosopher_using_monitor();
    sync::mpsc::test::test_all();

    loop {}
}

/// The entry point for another processors
#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub extern "C" fn other_main() -> ! {
    arch::gdt::init();
    arch::idt::init();
    arch::driver::apic::other_init();
    let cpu_id = arch::driver::apic::lapic_id();
    let ms = unsafe { arch::smp::notify_started(cpu_id) };
    unsafe { ms.activate(); }
    println!("Hello world! from CPU {}!", arch::driver::apic::lapic_id());
//    unsafe{ let a = *(0xdeadbeaf as *const u8); } // Page fault
    loop {}
}

/// Global heap allocator
///
/// Available after `memory::init()`.
///
/// It should be defined in memory mod, but in Rust `global_allocator` must be in root mod.
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
