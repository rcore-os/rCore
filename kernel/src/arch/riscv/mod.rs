use trapframe;

#[cfg(feature = "board_u540")]
#[path = "board/u540/mod.rs"]
pub mod board;
#[cfg(not(feature = "board_u540"))]
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
pub mod signal;
pub mod syscall;
pub mod timer;

use crate::memory::phys_to_virt;
use core::sync::atomic::{AtomicBool, Ordering};
use riscv::register::sie;

#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    let device_tree_vaddr = phys_to_virt(device_tree_paddr);

    unsafe {
        cpu::set_cpu_id(hartid);
    }

    if hartid != BOOT_HART_ID {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {}
        others_main(hartid);
    }

    unsafe {
        memory::clear_bss();
    }

    println!(
        "Hello RISCV! in hart {}, device tree @ {:#x}",
        hartid, device_tree_vaddr
    );

    crate::logging::init();
    unsafe {
        trapframe::init();
    }
    memory::init(device_tree_vaddr);
    timer::init();
    // FIXME: init driver on u540
    #[cfg(not(any(feature = "board_u540")))]
    board::init(device_tree_vaddr);
    unsafe {
        board::init_external_interrupt();
    }
    crate::process::init();

    AP_CAN_INIT.store(true, Ordering::Relaxed);
    crate::kmain();
}

fn others_main(hartid: usize) -> ! {
    unsafe {
        trapframe::init();
    }
    memory::init_other();
    timer::init();
    info!("Hello RISCV! in hart {}", hartid);
    crate::kmain();
}

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "board_u540"))]
const BOOT_HART_ID: usize = 0;
#[cfg(feature = "board_u540")]
const BOOT_HART_ID: usize = 1;

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("boot/entry32.asm"));
#[cfg(target_arch = "riscv64")]
global_asm!(include_str!("boot/entry64.asm"));

pub fn get_sp() -> usize {
    let sp: usize;
    unsafe {
        llvm_asm!("mv $0, sp" : "=r"(sp));
    }
    sp
}

pub fn set_sp(sp: usize) {
    unsafe {
        llvm_asm!("mv sp, $0" :: "r" (sp) : "memory");
    }
}
