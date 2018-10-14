use consts::MAX_CPU_NUM;
use core::ptr::{read_volatile, write_volatile};
use memory::*;

static mut STARTED: [bool; MAX_CPU_NUM] = [false; MAX_CPU_NUM];

pub unsafe fn set_cpu_id(cpu_id: usize) {
    unsafe {
        asm!("mv tp, $0" : : "r"(cpu_id));
    }
}

pub unsafe fn get_cpu_id() -> usize {
    let mut cpu_id = 0;
    unsafe {
        asm!("mv $0, tp" : : "r" (cpu_id));
    }
    cpu_id
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