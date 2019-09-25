use crate::arch::cpu;
use crate::arch::interrupt::TrapFrame;
use crate::process::*;
use log::*;

pub static mut TICK: usize = 0;

pub fn uptime_msec() -> usize {
    unsafe { crate::trap::TICK / crate::consts::USEC_PER_TICK / 1000 }
}

pub fn timer() {
    if cpu::id() == 0 {
        unsafe {
            TICK += 1;
        }
    }
    processor().tick();
}

pub fn error(tf: &TrapFrame) -> ! {
    error!("{:#x?}", tf);
    let tid = processor().tid();
    error!("On CPU{} Thread {}", cpu::id(), tid);
    let thread = unsafe { current_thread() };
    let mut proc=thread.proc.lock();
    proc.threads.retain(|&id| id != tid);
    if proc.threads.len()==0 {
        proc.exit(0x100);
    }    
    drop(proc);
    // TODO: futex wait, and make sure that no dangerous operation here.
    //  A better approach would be using a real signal...
    //  But I'm not inventing the world again.
    //  Anyway, you can emulate a signal receiver, and every thread kills itself when it receives a signal.
    //  At least this is a bit better than any cross-thread killing.
    processor().manager().exit(tid, 0x100);
    processor().yield_now();
    unreachable!();
}

pub fn serial(c: char) {
    if c == '\r' {
        // in linux, we use '\n' instead
        crate::fs::STDIN.push('\n');
    } else {
        crate::fs::STDIN.push(c);
    }
}
