pub mod consts;
pub mod cpu;
pub mod driver;
pub mod interrupt;
pub mod io;
pub mod memory;
pub mod paging;
pub mod rand;
pub mod syscall;
pub mod timer;

use log::*;
use mips::registers::cp0;

#[cfg(feature = "board_malta")]
#[path = "board/malta/mod.rs"]
pub mod board;

#[cfg(feature = "board_thinpad")]
#[path = "board/thinpad/mod.rs"]
pub mod board;

#[cfg(feature = "board_mipssim")]
#[path = "board/mipssim/mod.rs"]
pub mod board;

#[no_mangle]
pub extern "C" fn rust_main() -> ! {
    // unsafe { cpu::set_cpu_id(hartid); }

    let ebase = cp0::ebase::read_u32();
    let cpu_id = ebase & 0x3ff;
    let dtb_start = board::DTB.as_ptr() as usize;

    if cpu_id != BOOT_CPU_ID {
        // TODO: run others_main on other CPU
        // while unsafe { !cpu::has_started(hartid) }  { }
        // println!("Hello RISCV! in hart {}, dtb @ {:#x}", hartid, dtb);
        // others_main();
        loop {}
    }

    unsafe {
        memory::clear_bss();
    }

    board::init_serial_early();
    crate::logging::init();

    interrupt::init();
    memory::init();
    timer::init();
    driver::init();

    println!("Hello MIPS 32 from CPU {}, dtb @ {:#x}", cpu_id, dtb_start);

    crate::drivers::init(dtb_start);
    crate::process::init();

    // TODO: start other CPU
    // unsafe { cpu::start_others(hart_mask); }
    crate::kmain();
}

fn others_main() -> ! {
    interrupt::init();
    memory::init_other();
    timer::init();
    crate::kmain();
}

const BOOT_CPU_ID: u32 = 0;

global_asm!(include_str!("boot/context.gen.s"));
global_asm!(include_str!("boot/entry.gen.s"));
global_asm!(include_str!("boot/trap.gen.s"));
