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
#[cfg(target_arch = "x86_64")]
extern crate alloc;
#[cfg(target_arch = "x86_64")]
extern crate bit_allocator;
#[cfg(target_arch = "x86_64")]
extern crate bit_field;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate bitflags;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate lazy_static;
#[cfg(target_arch = "x86_64")]
extern crate linked_list_allocator;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate log;
#[cfg(target_arch = "x86_64")]
extern crate multiboot2;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate once;
extern crate rlibc;
#[cfg(target_arch = "x86_64")]
extern crate simple_filesystem;
extern crate spin;
#[cfg(target_arch = "x86_64")]
extern crate syscall as redox_syscall;
#[cfg(target_arch = "x86_64")]
extern crate uart_16550;
#[cfg(target_arch = "x86_64")]
extern crate ucore_memory;
#[cfg(target_arch = "x86_64")]
extern crate volatile;
#[macro_use]
#[cfg(target_arch = "x86_64")]
extern crate x86_64;
#[cfg(target_arch = "x86_64")]
extern crate xmas_elf;

pub use arch::interrupt::rust_trap;
#[cfg(target_arch = "x86_64")]
pub use arch::interrupt::set_return_rsp;
#[cfg(target_arch = "x86_64")]
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
#[cfg(target_arch = "x86_64")]
mod util;
#[macro_use]
mod macros;
#[cfg(target_arch = "x86_64")]
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
    test!(global_allocator);
    test!(guard_page);
    test!(find_mp);

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

    loop {}

    test_end!();
    unreachable!();
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
    ms.switch();
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
#[cfg(target_arch = "x86_64")]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::empty();

#[cfg(target_arch = "x86_64")]
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
        unsafe { asm!("int 3"::::"intel" "volatile"); }

        fn stack_overflow() {
            stack_overflow(); // for each recursion, the return address is pushed
        }

        // trigger a stack overflow
        stack_overflow();

        println!("It did not crash!");
    }
}