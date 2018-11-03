use consts::MAX_CPU_NUM;
use core::ptr::{read_volatile, write_volatile};
use memory::*;

static mut STARTED: [bool; MAX_CPU_NUM] = [false; MAX_CPU_NUM];

pub unsafe fn set_cpu_id(cpu_id: usize) {
    asm!("mv tp, $0" : : "r"(cpu_id));
}

pub fn id() -> usize {
    let cpu_id;
    unsafe { asm!("mv $0, tp" : "=r"(cpu_id)); }
    cpu_id
}

pub fn send_ipi(cpu_id: usize) {
    super::bbl::sbi::send_ipi(1 << cpu_id);
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
    use super::riscv::asm::wfi;
    unsafe { wfi() }
}