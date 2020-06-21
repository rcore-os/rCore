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

#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    let device_tree_vaddr = phys_to_virt(device_tree_paddr);

    unsafe {
        cpu::set_cpu_id(hartid);
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
    unsafe {
        trapframe::init();
    }
    memory::init(device_tree_vaddr);
    timer::init();
    // FIXME: init driver on u540
    #[cfg(not(any(feature = "board_u540")))]
    crate::drivers::init(device_tree_vaddr);
    #[cfg(not(feature = "board_k210"))]
    unsafe {
        board::init_external_interrupt();
    }
    crate::process::init();

    AP_CAN_INIT.store(true, Ordering::Relaxed);
    crate::kmain();
}

fn others_main() -> ! {
    unsafe {
        trapframe::init();
    }
    memory::init_other();
    timer::init();
    crate::kmain();
}

static AP_CAN_INIT: AtomicBool = AtomicBool::new(false);

#[cfg(not(feature = "board_u540"))]
const BOOT_HART_ID: usize = 0;
#[cfg(feature = "board_u540")]
const BOOT_HART_ID: usize = 1;

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("boot/entry32.asm"));
#[cfg(all(target_arch = "riscv64", not(feature = "board_k210")))]
global_asm!(include_str!("boot/entry64.asm"));
#[cfg(feature = "board_k210")]
global_asm!(include_str!("boot/entry_k210.asm"));

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
