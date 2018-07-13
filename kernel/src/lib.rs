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
extern crate log;
#[macro_use]
extern crate once;
extern crate rlibc;
#[cfg(target_arch = "x86_64")]
extern crate simple_filesystem;
extern crate spin;
extern crate ucore_memory;
extern crate volatile;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate x86_64;
extern crate xmas_elf;

// Export to asm
pub use arch::interrupt::rust_trap;
#[cfg(target_arch = "x86_64")]
pub use arch::interrupt::set_return_rsp;
#[cfg(target_arch = "x86_64")]
pub use arch::other_main;
use linked_list_allocator::LockedHeap;

#[macro_use]    // print!
mod logging;
mod memory;
mod lang;
mod util;
mod consts;
mod process;
#[cfg(target_arch = "x86_64")]
mod syscall;
#[cfg(target_arch = "x86_64")]
mod fs;
mod thread;
mod sync;
mod trap;

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
    logging::init();
    arch::init();
    process::init();
    info!("RISCV init end");

    #[cfg(feature = "link_user_program")]
        {
            use core::slice;
            let slice = unsafe {
                slice::from_raw_parts(_binary_hello_start as *const u8,
                                      _binary_hello_size as usize)
            };

            process::add_user_process("hello", slice);
            process::print();


            extern {
                fn _binary_hello_start();
                fn _binary_hello_size();
            }
        }

    unsafe { arch::interrupt::enable(); }
    loop {}
}

/// The entry point of Rust kernel
#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
    // ATTENTION: we have a very small stack and no guard page
    println!("Hello World{}", "!");

    logging::init();
    arch::init(multiboot_information_address);

    process::init();

    fs::load_sfs();

    unsafe { arch::interrupt::enable(); }

//    thread::test::local_key();
//    thread::test::unpack();
//    sync::test::philosopher_using_mutex();
//    sync::test::philosopher_using_monitor();
//    sync::mpsc::test::test_all();

    loop {}
}

/// Global heap allocator
///
/// Available after `memory::init()`.
///
/// It should be defined in memory mod, but in Rust `global_allocator` must be in root mod.
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();
