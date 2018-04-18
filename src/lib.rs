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
#![no_std]


#[macro_use]
extern crate alloc;

extern crate rlibc;
extern crate volatile;
extern crate spin;
extern crate multiboot2;
#[macro_use]
extern crate bitflags;
extern crate x86_64;
#[macro_use]
extern crate once;
extern crate linked_list_allocator;
#[macro_use]
extern crate lazy_static;
extern crate bit_field;
extern crate syscall;

#[macro_use]    // print!
mod io;
mod memory;
mod lang;
mod util;
#[macro_use]    // test!
mod test_util;
mod consts;

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
mod arch;

// The entry point of Rust kernel
#[no_mangle]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
    arch::cpu::init();

    // ATTENTION: we have a very small stack and no guard page
    println!("Hello World{}", "!");

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };

    // set up guard page and map the heap pages
    let mut memory_controller = memory::init(boot_info);    
    unsafe {
        use consts::{KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE};
        HEAP_ALLOCATOR.lock().init(KERNEL_HEAP_OFFSET, KERNEL_HEAP_SIZE);
    }

    arch::gdt::init();
    arch::idt::init();

    test!(global_allocator);
    test!(guard_page);
    test!(find_mp);

    let acpi = arch::driver::init(
        |addr: usize| memory_controller.map_page_identity(addr));
    // memory_controller.print_page_table();
    arch::smp::start_other_cores(&acpi, &mut memory_controller);

    unsafe{ arch::interrupt::enable(); }
    loop{}

    test_end!();
    unreachable!();
}

#[no_mangle]
pub extern "C" fn other_main() -> ! {
    arch::cpu::init();
    arch::gdt::init();
    arch::idt::init();
    arch::driver::apic::other_init();
    // FIXME: deadlock!
    println!("Hello world! from CPU {}!", arch::driver::apic::lapic_id());
    unsafe{ let a = *(0xdeadbeaf as *const u8); } // Page fault
    loop {}
}

use linked_list_allocator::LockedHeap;

#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

mod test {
    pub fn global_allocator() {
        for i in 0..10000 {
            format!("Some String");
        }
    }
    pub fn find_mp() {
        use arch;
        let mp = arch::driver::mp::find_mp();
        assert!(mp.is_some());
    }
    pub fn guard_page() {
        use x86_64;
        // invoke a breakpoint exception
        x86_64::instructions::interrupts::int3();

        fn stack_overflow() {
            stack_overflow(); // for each recursion, the return address is pushed
        }

        // trigger a stack overflow
        stack_overflow();

        println!("It did not crash!");
    }
}