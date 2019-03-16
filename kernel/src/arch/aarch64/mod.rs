//! Entrance and initialization for aarch64.

pub mod io;
pub mod paging;
pub mod memory;
pub mod interrupt;
pub mod consts;
pub mod cpu;
pub mod driver;
pub mod timer;
pub mod syscall;

#[cfg(feature = "board_raspi3")]
#[path = "board/raspi3/mod.rs"]
pub mod board;

global_asm!(include_str!("boot/boot.S"));

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn rust_main() -> ! {
    memory::init_mmu_early(); // Enable mmu and paging
    board::init_serial_early();

    crate::logging::init();
    interrupt::init();
    memory::init();
    driver::init();
    println!("{}", LOGO);

    crate::process::init();

    crate::kmain();
}

const LOGO: &str = r#"
    ____                __   ____  _____
   / __ \ __  __ _____ / /_ / __ \/ ___/
  / /_/ // / / // ___// __// / / /\__ \
 / _, _// /_/ /(__  )/ /_ / /_/ /___/ /
/_/ |_| \__,_//____/ \__/ \____//____/
"#;
