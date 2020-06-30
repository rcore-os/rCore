//! Entrance and initialization for aarch64.

use core::sync::atomic::{spin_loop_hint, AtomicBool, Ordering};

mod boot;
pub mod consts;
pub mod cpu;
pub mod fp;
pub mod interrupt;
pub mod io;
pub mod memory;
pub mod paging;
pub mod rand;
pub mod signal;
pub mod syscall;
pub mod timer;

#[cfg(feature = "board_raspi3")]
#[path = "board/raspi3/mod.rs"]
pub mod board;

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn main_start() -> ! {
    // start up other CPUs
    unsafe { cpu::start_others() };

    crate::logging::init();
    unsafe {
        trapframe::init();
    }
    memory::init();

    board::early_init();
    println!("Hello {}! from CPU {}", board::BOARD_NAME, cpu::id());

    board::early_final();
    crate::lkm::manager::ModuleManager::init();
    board::init();

    crate::process::init();

    // wake up other CPUs
    AP_CAN_INIT.store(true, Ordering::Relaxed);

    crate::kmain();
}

#[no_mangle]
pub extern "C" fn others_start() -> ! {
    println!("Hello {}! from CPU {}", board::BOARD_NAME, cpu::id());

    while !AP_CAN_INIT.load(Ordering::Relaxed) {
        spin_loop_hint();
    }

    unsafe {
        trapframe::init();
    }
    memory::init_other();
    //timer::init();
    crate::kmain();
}
