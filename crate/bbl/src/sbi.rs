//! Port from sbi.h

#[inline(always)]
fn sbi_call(which: u32, arg0: u32, arg1: u32, arg2: u32) -> u32 {
    let ret;
    unsafe {
        asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (which)
            : "memory"
            : "volatile");
    }
    ret
}

pub fn console_putchar(ch: u32) {
    sbi_call(SBI_CONSOLE_PUTCHAR, ch, 0, 0);
}

pub fn console_getchar() -> u32 {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
}

pub fn set_timer(stime_value: u64) {
    sbi_call(SBI_SET_TIMER, stime_value as u32, (stime_value >> 32) as u32, 0);
}

pub fn clear_ipi() {
    sbi_call(SBI_CLEAR_IPI, 0, 0, 0);
}

pub fn send_ipi(hart_mask: *const u32) {
    sbi_call(SBI_SEND_IPI, hart_mask as u32, 0, 0);
}

pub fn remote_fence_i(hart_mask: *const u32) {
    sbi_call(SBI_REMOTE_FENCE_I, hart_mask as u32, 0, 0);
}

pub fn remote_sfence_vma(hart_mask: *const u32, _start: u32, _size: u32) {
    sbi_call(SBI_REMOTE_SFENCE_VMA, hart_mask as u32, 0, 0);
}

pub fn remote_sfence_vma_asid(hart_mask: *const u32, _start: u32, _size: u32, _asid: u32) {
    sbi_call(SBI_REMOTE_SFENCE_VMA_ASID, hart_mask as u32, 0, 0);
}

const SBI_SET_TIMER: u32 = 0;
const SBI_CONSOLE_PUTCHAR: u32 = 1;
const SBI_CONSOLE_GETCHAR: u32 = 2;
const SBI_CLEAR_IPI: u32 = 3;
const SBI_SEND_IPI: u32 = 4;
const SBI_REMOTE_FENCE_I: u32 = 5;
const SBI_REMOTE_SFENCE_VMA: u32 = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: u32 = 7;
const SBI_SHUTDOWN: u32 = 8;
