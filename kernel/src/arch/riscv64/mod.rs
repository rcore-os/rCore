pub mod io;
pub mod interrupt;
pub mod timer;
pub mod paging;
pub mod memory;
pub mod compiler_rt;
pub mod consts;
pub mod cpu;

#[no_mangle]
pub extern fn rust_main(hartid: usize, dtb: usize, hart_mask: usize, functions: usize) -> ! {
    unsafe { cpu::set_cpu_id(hartid); }
    unsafe { BBL_FUNCTIONS_PTR = functions as *const _; }
    println!("Hello RISCV! in hart {}, dtb @ {:#x}, functions @ {:#x}", hartid, dtb, functions);

    if hartid != 0 {
        while unsafe { !cpu::has_started(hartid) }  { }
        others_main();
        unreachable!();
    }

    crate::logging::init();
    interrupt::init();
    memory::init();
    timer::init();
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


/// Constant & Macro for `trap.asm`
#[cfg(feature = "m_mode")]
global_asm!("
    .equ xstatus,   0x300
    .equ xscratch,  0x340
    .equ xepc,      0x341
    .equ xcause,    0x342
    .equ xtval,     0x343
    .macro XRET\n mret\n .endm
");
#[cfg(not(feature = "m_mode"))]
global_asm!("
    .equ xstatus,   0x100
    .equ xscratch,  0x140
    .equ xepc,      0x141
    .equ xcause,    0x142
    .equ xtval,     0x143
    .macro XRET\n sret\n .endm
");

#[cfg(feature = "no_bbl")]
global_asm!(include_str!("boot/boot.asm"));
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