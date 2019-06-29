use crate::consts::MAX_CPU_NUM;
use core::ptr::{read_volatile, write_volatile};
use mips::instructions;
use mips::registers::cp0;

static mut STARTED: [bool; MAX_CPU_NUM] = [false; MAX_CPU_NUM];

pub fn id() -> usize {
    (cp0::ebase::read_u32() as usize) & 0x3ff
}

pub unsafe fn has_started(cpu_id: usize) -> bool {
    read_volatile(&STARTED[cpu_id])
}

pub unsafe fn start_others(hart_mask: usize) {
    for cpu_id in 0..MAX_CPU_NUM {
        if (hart_mask >> cpu_id) & 1 != 0 {
            write_volatile(&mut STARTED[cpu_id], true);
        }
    }
}

pub fn halt() {
    unsafe {
        instructions::wait();
    }
}

pub unsafe fn exit_in_qemu(error_code: u8) -> ! {
    /* nothing to do */
    loop {}
}

pub unsafe fn reboot() -> ! {
    /* nothing to do */
    loop {}
}
