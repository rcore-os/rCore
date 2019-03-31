pub mod io;
//pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;
pub mod consts;
pub mod cpu;
pub mod syscall;
pub mod rand;
#[cfg(feature = "board_u540")]
#[path = "board/u540/mod.rs"]
mod board;
mod sbi;

use log::*;

#[no_mangle]
pub extern fn rust_main(hartid: usize, dtb: usize, hart_mask: usize) -> ! {
    // An initial recursive page table has been set by BBL (shared by all cores)

    unsafe { cpu::set_cpu_id(hartid); }

    if hartid != BOOT_HART_ID {
        while unsafe { !cpu::has_started(hartid) }  { }
        println!("Hello RISCV! in hart {}, dtb @ {:#x}", hartid, dtb);
        others_main();
        //other_main -> !
    }

    unsafe { memory::clear_bss(); }

    println!("Hello RISCV! in hart {}, dtb @ {:#x}", hartid, dtb);

    crate::logging::init();
    interrupt::init();
    memory::init(dtb);
    timer::init();
    // FIXME: init driver on u540
    #[cfg(not(feature = "board_u540"))]
    crate::drivers::init(dtb);
    #[cfg(feature = "board_u540")]
    unsafe { board::init_external_interrupt(); }
    crate::process::init();

    unsafe { cpu::start_others(hart_mask); }
    crate::kmain();
}

fn others_main() -> ! {
    interrupt::init();
    memory::init_other();
    timer::init();
    crate::kmain();
}

#[cfg(not(feature = "board_u540"))]
const BOOT_HART_ID: usize = 0;
#[cfg(feature = "board_u540")]
const BOOT_HART_ID: usize = 1;

/// Constant & Macro for `trap.asm`
#[cfg(target_arch = "riscv32")]
global_asm!(r"
    .equ XLENB,     4
    .equ XLENb,     32
    .macro LOAD a1, a2
        lw \a1, \a2*XLENB(sp)
    .endm
    .macro STORE a1, a2
        sw \a1, \a2*XLENB(sp)
    .endm
");
#[cfg(target_arch = "riscv64")]
global_asm!(r"
    .equ XLENB,     8
    .equ XLENb,     64
    .macro LOAD a1, a2
        ld \a1, \a2*XLENB(sp)
    .endm
    .macro STORE a1, a2
        sd \a1, \a2*XLENB(sp)
    .endm
");


global_asm!(include_str!("boot/entry.asm"));
global_asm!(include_str!("boot/trap.asm"));
