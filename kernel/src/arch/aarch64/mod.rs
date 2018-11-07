//! Entrance and initialization for aarch64.

extern crate atags;
extern crate cortex_a;

pub mod io;
pub mod paging;
pub mod memory;
pub mod interrupt;

#[cfg(feature = "board_raspi3")]
#[path = "board/raspi3/mod.rs"]
pub mod board;

pub use self::board::timer;

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn rust_main() -> ! {
    // Init board to enable serial port.
    board::init();
    ::logging::init(); // FIXME
    interrupt::init();
    memory::init();
    timer::init();
    ::kmain();
}

global_asm!(include_str!("boot/boot.S"));
