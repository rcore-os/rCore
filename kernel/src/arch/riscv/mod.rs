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
pub mod fp;
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
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use riscv::register::sie;

fn start_all_harts() {
    // simply wake up the first 64 harts.
    use sbi::sbi_hart_start;
    for i in 0..64 {
        let ret = sbi_hart_start(i, 0x80200000usize, i);
        info!("Start {}: {:?}", i, ret);
    }
}

#[no_mangle]
pub extern "C" fn rust_main(hartid: usize, device_tree_paddr: usize) -> ! {
    let device_tree_vaddr = phys_to_virt(device_tree_paddr);

    unsafe {
        cpu::set_cpu_id(hartid);
    }

    if FIRST_HART
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        LOTTERY_HART_ID.store(hartid, Ordering::SeqCst);
        start_all_harts();
    }
    let main_hart = LOTTERY_HART_ID.load(Ordering::SeqCst);
    if hartid != main_hart {
        while !AP_CAN_INIT.load(Ordering::Relaxed) {}
        others_main(hartid);
    }

    unsafe {
        memory::clear_bss();
    }
    crate::logging::init();

    unsafe {
        trapframe::init();
    }
    memory::init(device_tree_vaddr);
    timer::init();
    // TODO: init driver on u540
    #[cfg(not(any(feature = "board_u540")))]
    board::init(device_tree_vaddr);
    unsafe {
        board::init_external_interrupt();
    }
    crate::process::init();
    info!(
        "Hello RISCV! in hart {}, device tree @ {:#x}",
        hartid, device_tree_vaddr
    );
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
static FIRST_HART: AtomicBool = AtomicBool::new(false);
static LOTTERY_HART_ID: AtomicUsize = AtomicUsize::new(0);

#[cfg(not(feature = "board_u540"))]
const BOOT_HART_ID: usize = 0;
#[cfg(feature = "board_u540")]
const BOOT_HART_ID: usize = 1;

#[cfg(target_arch = "riscv32")]
global_asm!(include_str!("boot/entry32.asm"));
#[cfg(target_arch = "riscv64")]
global_asm!(include_str!("boot/entry64.asm"));
