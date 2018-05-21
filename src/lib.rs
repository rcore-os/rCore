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
#![no_std]


#[macro_use]
extern crate alloc;

extern crate rlibc;
extern crate volatile;
extern crate spin;
extern crate multiboot2;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate x86_64;
#[macro_use]
extern crate once;
extern crate linked_list_allocator;
#[macro_use]
extern crate lazy_static;
extern crate bit_field;
extern crate syscall as redox_syscall;
extern crate xmas_elf;
extern crate arrayvec;
#[macro_use]
extern crate log;
extern crate simple_filesystem;

#[macro_use]    // print!
mod io;
mod memory;
mod lang;
mod util;
#[macro_use]
mod macros;
mod consts;
mod process;
mod syscall;
mod fs;

#[allow(dead_code)]
#[cfg(target_arch = "x86_64")]
#[path = "arch/x86_64/mod.rs"]
mod arch;

/// The entry point of Rust kernel
#[no_mangle]
pub extern "C" fn rust_main(multiboot_information_address: usize) -> ! {
    arch::cpu::init();
    io::init();

    // ATTENTION: we have a very small stack and no guard page
    println!("Hello World{}", "!");

    let boot_info = unsafe { multiboot2::load(multiboot_information_address) };

    // set up guard page and map the heap pages
    let mut memory_controller = memory::init(boot_info);

    arch::gdt::init();
    arch::idt::init();

    test!(cow);
    test!(global_allocator);
    test!(guard_page);
    test!(find_mp);

    let acpi = arch::driver::init(
        |addr: usize| memory_controller.map_page_identity(addr));
    // memory_controller.print_page_table();

    arch::smp::start_other_cores(&acpi, &mut memory_controller);
    process::init(memory_controller);

    fs::load_sfs();

    unsafe{ arch::interrupt::enable(); }

    // 直接进入用户态暂不可用：内核代码用户不可访问
//    unsafe{
//        use arch::syscall;
//        // 在用户模式下触发时钟中断，会导致GPF
//        // （可能是由于没有合理分离栈）
//        no_interrupt!({
//            syscall::switch_to_user();
//            println!("Now in user mode");
//            syscall::switch_to_kernel();
//            println!("Now in kernel mode");
//        });
//    }

    loop{
        println!("init ...");
        let mut i = 0;
        while i < 1 << 22 {
            i += 1;
        }
    }

    test_end!();
    unreachable!();
}

/// The entry point for another processors
#[no_mangle]
pub extern "C" fn other_main() -> ! {
    arch::cpu::init();
    arch::gdt::init();
    arch::idt::init();
    arch::driver::apic::other_init();
    let cpu_id = arch::driver::apic::lapic_id();
    println!("Hello world! from CPU {}!", arch::driver::apic::lapic_id());
    unsafe{ arch::smp::notify_started(cpu_id); }
//    unsafe{ let a = *(0xdeadbeaf as *const u8); } // Page fault
    loop {}
}

pub use arch::interrupt::rust_trap;

use linked_list_allocator::LockedHeap;

/// Global heap allocator
///
/// Available after `memory::init()`.
///
/// It should be defined in memory mod, but in Rust `global_allocator` must be in root mod.
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

    pub fn cow() {
        use arch;
        arch::paging::test_cow();
    }
}