pub mod io;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;
pub mod consts;
pub mod cpu;
pub mod syscall;
pub mod rand;
pub mod driver;

use log::*;
use mips::registers::cp0;
use mips::instructions;

#[cfg(feature = "board_malta")]
#[path = "board/malta/mod.rs"]
pub mod board;

#[cfg(feature = "board_thinpad")]
#[path = "board/thinpad/mod.rs"]
pub mod board;

extern "C" {
    fn _dtb_start();
    fn _dtb_end();
}

#[no_mangle]
pub extern fn rust_main() -> ! {

    // unsafe { cpu::set_cpu_id(hartid); }

    let ebase = cp0::ebase::read_u32();
    let cpu_id = ebase & 0x3ff;
    let dtb_start = _dtb_start as usize;
    let dtb_end = _dtb_end as usize;

    if cpu_id != BOOT_CPU_ID {
        // TODO: run others_main on other CPU
        // while unsafe { !cpu::has_started(hartid) }  { }
        // println!("Hello RISCV! in hart {}, dtb @ {:#x}", hartid, dtb);
        // others_main();
        loop {}
    }

    unsafe { memory::clear_bss(); }

    board::init_serial_early();
    driver::init();

    println!("Hello MIPS 32 from CPU {}, dtb @ {:#x}", cpu_id, dtb_start);

    interrupt::init();
    memory::init();
    timer::init();

    crate::logging::init();
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

global_asm!(include_str!("boot/entry.gen.s"));
global_asm!(include_str!("boot/trap.gen.s"));
global_asm!(include_str!("boot/dtb.gen.s"));
