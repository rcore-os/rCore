use super::riscv::register::*;
pub use self::context::*;

#[path = "context.rs"]
mod context;

pub fn init() {
    extern {
        fn __alltraps();
    }
    unsafe {
        // Set sscratch register to 0, indicating to exception vector that we are
        // presently executing in the kernel
        sscratch::write(0);
        // Set the exception vector address
        stvec::write(__alltraps as usize, stvec::TrapMode::Direct);
        // Enable IPI
        sie::set_ssoft();
    }
    info!("interrupt: init end");
}

#[inline(always)]
pub unsafe fn enable() {
    sstatus::set_sie();
}

#[inline(always)]
pub unsafe fn disable_and_store() -> usize {
    let e = sstatus::read().sie() as usize;
    sstatus::clear_sie();
    e
}

#[inline(always)]
pub unsafe fn restore(flags: usize) {
    if flags != 0 {
        sstatus::set_sie();
    }
}

#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) {
    use super::riscv::register::scause::{Trap, Interrupt as I, Exception as E};
    trace!("Interrupt: {:?}", tf.scause.cause());
    match tf.scause.cause() {
        Trap::Interrupt(I::SupervisorSoft) => ipi(),
        Trap::Interrupt(I::SupervisorTimer) => timer(),
        Trap::Exception(E::IllegalInstruction) => illegal_inst(tf),
        Trap::Exception(E::UserEnvCall) => syscall(tf),
        _ => ::trap::error(tf),
    }
    ::trap::before_return();
    trace!("Interrupt end");
}

fn ipi() {
    debug!("IPI");
    super::bbl::sbi::clear_ipi();
}

fn timer() {
    ::trap::timer();
    super::timer::set_next();
}

fn syscall(tf: &mut TrapFrame) {
    tf.sepc += 4;   // Must before syscall, because of fork.
    let ret = ::syscall::syscall(tf.x[10], [tf.x[11], tf.x[12], tf.x[13], tf.x[14], tf.x[15], tf.x[16]], tf);
    tf.x[10] = ret as usize;
}

fn illegal_inst(tf: &mut TrapFrame) {
    if !emulate_mul_div(tf) {
        ::trap::error(tf);
    }
}

/// Migrate from riscv-pk
fn emulate_mul_div(tf: &mut TrapFrame) -> bool {
    let insn = unsafe { *(tf.sepc as *const usize) };
    let rs1 = tf.x[get_reg(insn, RS1)];
    let rs2 = tf.x[get_reg(insn, RS2)];

    let rd = if (insn & MASK_MUL) == MATCH_MUL {
        rs1 * rs2
    } else if (insn & MASK_DIV) == MATCH_DIV {
        ((rs1 as i32) / (rs2 as i32)) as usize
    } else if (insn & MASK_DIVU) == MATCH_DIVU {
        rs1 / rs2
    } else if (insn & MASK_REM) == MATCH_REM {
        ((rs1 as i32) % (rs2 as i32)) as usize
    } else if (insn & MASK_REMU) == MATCH_REMU {
        rs1 % rs2
    } else if (insn & MASK_MULH) == MATCH_MULH {
        (((rs1 as i32 as i64) * (rs2 as i32 as i64)) >> 32) as usize
    } else if (insn & MASK_MULHU) == MATCH_MULHU {
        (((rs1 as i64) * (rs2 as i64)) >> 32) as usize
    } else if (insn & MASK_MULHSU) == MATCH_MULHSU {
        (((rs1 as i32 as i64) * (rs2 as i64)) >> 32) as usize
    } else {
        return false;
    };
    tf.x[get_reg(insn, RD)] = rd;
    tf.sepc += 4;
    return true;

    fn get_reg(inst: usize, offset: usize) -> usize {
        (inst >> offset) & 0x1f
    }

    const RS1: usize = 15;
    const RS2: usize = 20;
    const RD: usize = 7;

    const MATCH_MUL: usize = 0x2000033;
    const MASK_MUL: usize = 0xfe00707f;
    const MATCH_MULH: usize = 0x2001033;
    const MASK_MULH: usize = 0xfe00707f;
    const MATCH_MULHSU: usize = 0x2002033;
    const MASK_MULHSU: usize = 0xfe00707f;
    const MATCH_MULHU: usize = 0x2003033;
    const MASK_MULHU: usize = 0xfe00707f;
    const MATCH_DIV: usize = 0x2004033;
    const MASK_DIV: usize = 0xfe00707f;
    const MATCH_DIVU: usize = 0x2005033;
    const MASK_DIVU: usize = 0xfe00707f;
    const MATCH_REM: usize = 0x2006033;
    const MASK_REM: usize = 0xfe00707f;
    const MATCH_REMU: usize = 0x2007033;
    const MASK_REMU: usize = 0xfe00707f;
}