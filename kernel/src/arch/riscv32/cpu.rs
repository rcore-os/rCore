use crate::consts::MAX_CPU_NUM;
use core::ptr::{read_volatile, write_volatile};

static mut STARTED: [bool; MAX_CPU_NUM] = [false; MAX_CPU_NUM];

pub unsafe fn set_cpu_id(cpu_id: usize) {
    asm!("mv gp, $0" : : "r"(cpu_id));
}

pub fn id() -> usize {
    let cpu_id;
    unsafe {
        asm!("mv $0, gp" : "=r"(cpu_id));
    }
    cpu_id
}

pub fn send_ipi(cpu_id: usize) {
    super::sbi::send_ipi(1 << cpu_id);
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
    unsafe { riscv::asm::wfi() }
}

pub unsafe fn exit_in_qemu(error_code: u8) -> ! {
    super::sbi::shutdown()
}
