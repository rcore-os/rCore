extern crate riscv;
extern crate bbl;

pub mod io;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;
pub mod consts;
pub mod smp;

use self::smp::*;

fn others_main(hartid: usize) -> ! {
    println!("hart {} is booting", hartid);
    loop { }
}

#[no_mangle]
pub extern fn rust_main(hartid: usize, dtb: usize, hart_mask: usize) -> ! {
    unsafe { set_cpu_id(hartid); } 

    if hartid != 0 {
        while unsafe { !has_started(hartid) }  { }
        others_main(hartid);
        // others_main should not return
    }

    println!("Hello RISCV! in hart {}, {}, {}", hartid, dtb, hart_mask);

    ::logging::init();
    interrupt::init();
    memory::init();
    timer::init();

    unsafe { start_others(hart_mask); }
    ::kmain();
}

#[cfg(feature = "no_bbl")]
global_asm!(include_str!("boot/boot.asm"));
global_asm!(include_str!("boot/entry.asm"));
global_asm!(include_str!("boot/trap.asm"));