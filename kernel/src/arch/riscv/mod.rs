#[cfg(feature = "board_u540")]
#[path = "board/u540/mod.rs"]
pub mod board;
#[cfg(feature = "board_rocket_chip")]
#[path = "board/rocket_chip/mod.rs"]
pub mod board;
#[cfg(not(any(feature = "board_u540", feature = "board_rocket_chip")))]
#[path = "board/virt/mod.rs"]
pub mod board;

pub mod compiler_rt;
pub mod consts;
pub mod cpu;
pub mod interrupt;
pub mod io;
pub mod memory;
pub mod paging;
pub mod rand;
mod sbi;
pub mod syscall;
pub mod timer;

use crate::memory::phys_to_virt;
use core::sync::atomic::{AtomicBool, Ordering};
use log::*;

#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    let mut device_tree_vaddr = phys_to_virt(device_tree_paddr);

    unsafe {
        cpu::set_cpu_id(hartid);
    }

    #[cfg(feature = "board_rocket_chip")]
    {
        device_tree_vaddr = board::DTB.as_ptr() as usize;
    }

    if hartid != BOOT_HART_ID {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {}
        println!(
            "Hello RISCV! in hart {}, device tree @ {:#x}",
            hartid, device_tree_vaddr
        );
        others_main();
        //other_main -> !
    }

    unsafe {
        memory::clear_bss();
    }

    println!(
        "Hello RISCV! in hart {}, device tree @ {:#x}",
        hartid, device_tree_vaddr
    );

    crate::logging::init();
    interrupt::init();
    memory::init(device_tree_vaddr);
    timer::init();
    // FIXME: init driver on u540
    #[cfg(not(any(feature = "board_u540")))]
    crate::drivers::init(device_tree_vaddr);
    #[cfg(not(feature = "board_k210"))]
    unsafe {
        board::enable_serial_interrupt();
        board::init_external_interrupt();
    }
    crate::process::init();

    AP_CAN_INIT.store(true, Ordering::Relaxed);
    crate::kmain();
}

fn others_main() -> ! {
    interrupt::init();
    memory::init_other();
    timer::init();
    crate::kmain();
}

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "board_u540"))]
const BOOT_HART_ID: usize = 0;
#[cfg(feature = "board_u540")]
const BOOT_HART_ID: usize = 1;

/// Constant & Macro for `trap.asm`
#[cfg(target_arch = "riscv32")]
global_asm!(
    r"
    .equ XLENB,     4
    .equ XLENb,     32
    .macro LOAD a1, a2
        lw \a1, \a2*XLENB(sp)
    .endm
    .macro STORE a1, a2
        sw \a1, \a2*XLENB(sp)
    .endm
"
);
#[cfg(target_arch = "riscv64")]
global_asm!(
    r"
    .equ XLENB,     8
    .equ XLENb,     64
    .macro LOAD a1, a2
        ld \a1, \a2*XLENB(sp)
    .endm
    .macro STORE a1, a2
        sd \a1, \a2*XLENB(sp)
    .endm
"
);
#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("boot/entry32.asm"));
#[cfg(all(target_arch = "riscv64", not(feature = "board_k210")))]
global_asm!(include_str!("boot/entry64.asm"));
#[cfg(feature = "board_k210")]
global_asm!(include_str!("boot/entry_k210.asm"));
global_asm!(include_str!("boot/trap.asm"));
