pub mod io;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;
pub mod consts;
pub mod cpu;
pub mod syscall;
mod sbi;

use log::*;

#[no_mangle]
pub extern fn rust_main(hartid: usize, dtb: usize, hart_mask: usize, functions: usize) -> ! {
    // An initial recursive page table has been set by BBL (shared by all cores)

    unsafe { cpu::set_cpu_id(hartid); }

    if hartid != BOOT_HART_ID {
        while unsafe { !cpu::has_started(hartid) }  { }
        println!("Hello RISCV! in hart {}, dtb @ {:#x}, functions @ {:#x}", hartid, dtb, functions);
        others_main();
        //other_main -> !
    }

    unsafe { memory::clear_bss(); }
    unsafe { BBL_FUNCTIONS_PTR = functions as *const _; }

    println!("Hello RISCV! in hart {}, dtb @ {:#x}, functions @ {:#x}", hartid, dtb, functions);

    crate::logging::init();
    interrupt::init();
    memory::init(dtb);
    timer::init();
    // FIXME: init driver on u540
    #[cfg(not(feature = "board_u540"))]
    crate::drivers::init(dtb);
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
#[cfg(feature = "m_mode")]
global_asm!("
    .equ xstatus,   0x300
    .equ xscratch,  0x340
    .equ xepc,      0x341
    .equ xcause,    0x342
    .equ xtval,     0x343
    .macro XRET\n mret\n .endm
    .macro TEST_BACK_TO_KERNEL  // s0 == back to kernel?
        li   s3, 3 << 11
        and  s0, s1, s3         // mstatus.MPP = 3
    .endm
");
#[cfg(not(feature = "m_mode"))]
global_asm!("
    .equ xstatus,   0x100
    .equ xscratch,  0x140
    .equ xepc,      0x141
    .equ xcause,    0x142
    .equ xtval,     0x143
    .macro XRET\n sret\n .endm
    .macro TEST_BACK_TO_KERNEL
        andi s0, s1, 1 << 8     // sstatus.SPP = 1
    .endm
");

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


#[cfg(feature = "board_k210")]
global_asm!(include_str!("board/k210/boot.asm"));
global_asm!(include_str!("boot/entry.asm"));
global_asm!(include_str!("boot/trap.asm"));


/// Some symbols passed from BBL.
/// Used in M-mode kernel.
#[repr(C)]
struct BBLFunctions {
    mcall_trap: BBLTrapHandler,
    illegal_insn_trap: BBLTrapHandler,
    mcall_console_putchar: extern fn(u8),
    mcall_console_getchar: extern fn() -> usize,
}

type BBLTrapHandler = extern fn(regs: *const usize, mcause: usize, mepc: usize);
static mut BBL_FUNCTIONS_PTR: *const BBLFunctions = ::core::ptr::null();
use lazy_static::lazy_static;
lazy_static! {
    static ref BBL: BBLFunctions = unsafe { BBL_FUNCTIONS_PTR.read() };
}
