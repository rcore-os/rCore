//! Entrance and initialization for aarch64.

pub mod io;
pub mod paging;
pub mod memory;
pub mod interrupt;

#[cfg(feature = "board_raspi3")]
#[path = "board/raspi3/mod.rs"]
pub mod board;

/// TODO
/// The entry point of kernel
#[no_mangle]    // don't mangle the name of this function
pub extern fn rust_main() -> ! {
    println!("Hello ARM64!");

    // First init log mod, so that we can print log info.
    // ::logging::init();
    // Init trap handling.
    // interrupt::init();
    // Init physical memory management and heap.
    // memory::init();
    // Now heap is available
    // timer::init();

    // Init board.
    board::init();
    ::kmain();
}

// global_asm!(include_str!("boot/boot.asm"));
// global_asm!(include_str!("boot/entry.asm"));
// global_asm!(include_str!("boot/trap.asm"));
