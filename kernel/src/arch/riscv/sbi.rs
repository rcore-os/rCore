//! Port from sbi.h
#![allow(dead_code)]

#[derive(Clone, Copy, Debug)]
pub struct SBIRet {
    error: isize,
    value: usize,
}
#[derive(Clone, Copy, Debug)]
pub struct SBICall {
    eid: usize,
    fid: usize,
}
#[inline(always)]
fn sbi_call(which: SBICall, arg0: usize, arg1: usize, arg2: usize) -> SBIRet {
    let ret1;
    let ret2;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret1), "={x11}"(ret2)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (which.eid), "{x16}" (which.fid)
            : "memory"
            : "volatile");
    }
    SBIRet {
        error: ret1,
        value: ret2,
    }
}

pub fn sbi_hart_start(hartid: usize, start_addr: usize, opaque: usize) -> SBIRet {
    sbi_call(
        SBICall {
            eid: SBI_EID_HSM,
            fid: SBI_FID_HSM_START,
        },
        hartid,
        start_addr as usize,
        opaque,
    )
}
pub fn sbi_hart_stop() -> ! {
    sbi_call(
        SBICall {
            eid: SBI_EID_HSM,
            fid: SBI_FID_HSM_STOP,
        },
        0,
        0,
        0,
    );
    unreachable!();
}
pub fn sbi_hart_get_status(hartid: usize) -> SBIRet {
    sbi_call(
        SBICall {
            eid: SBI_EID_HSM,
            fid: SBI_FID_HSM_START,
        },
        hartid,
        0,
        0,
    )
}

pub fn sbi_set_timer(stime_value: u64) -> SBIRet {
    #[cfg(target_pointer_width = "32")]
    let ret = sbi_call(
        SBICall {
            eid: SBI_EID_TIME,
            fid: SBI_FID_TIME_SET,
        },
        stime_value as usize,
        (stime_value >> 32) as usize,
        0,
    );
    #[cfg(target_pointer_width = "64")]
    let ret = sbi_call(
        SBICall {
            eid: SBI_EID_TIME,
            fid: SBI_FID_TIME_SET,
        },
        stime_value as usize,
        0,
        0,
    );
    ret
}

const SBI_SUCCESS: isize = 0;
const SBI_ERR_FAILED: isize = -1;
const SBI_ERR_NOT_SUPPORTED: isize = -2;
const SBI_ERR_INVALID_PARAM: isize = -3;
const SBI_ERR_DENIED: isize = -4;
const SBI_ERR_INVALID_ADDRESS: isize = -5;
const SBI_ERR_ALREADY_AVAILABLE: isize = -6;

const SBI_EID_HSM: usize = 0x48534D;
const SBI_FID_HSM_START: usize = 0;
const SBI_FID_HSM_STOP: usize = 1;
const SBI_FID_HSM_STATUS: usize = 2;
const SBI_EID_TIME: usize = 0x54494D45;
const SBI_FID_TIME_SET: usize = 0;

/// Legacy calls.

#[inline(always)]
fn sbi_call_legacy(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
    let ret;
    unsafe {
        llvm_asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (arg0), "{x11}" (arg1), "{x12}" (arg2), "{x17}" (which)
            : "memory"
            : "volatile");
    }
    ret
}

pub fn console_putchar(ch: usize) {
    sbi_call_legacy(SBI_CONSOLE_PUTCHAR, ch, 0, 0);
}

pub fn console_getchar() -> usize {
    sbi_call_legacy(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

pub fn shutdown() -> ! {
    sbi_call_legacy(SBI_SHUTDOWN, 0, 0, 0);
    unreachable!()
}

pub fn set_timer(stime_value: u64) {
    #[cfg(target_pointer_width = "32")]
    sbi_call_legacy(
        SBI_SET_TIMER,
        stime_value as usize,
        (stime_value >> 32) as usize,
        0,
    );
    #[cfg(target_pointer_width = "64")]
    sbi_call_legacy(SBI_SET_TIMER, stime_value as usize, 0, 0);
}

pub fn clear_ipi() {
    sbi_call_legacy(SBI_CLEAR_IPI, 0, 0, 0);
}

pub fn send_ipi(hart_mask: usize) {
    sbi_call_legacy(SBI_SEND_IPI, &hart_mask as *const _ as usize, 0, 0);
}

pub fn remote_fence_i(hart_mask: usize) {
    sbi_call_legacy(SBI_REMOTE_FENCE_I, &hart_mask as *const _ as usize, 0, 0);
}

pub fn remote_sfence_vma(hart_mask: usize, _start: usize, _size: usize) {
    sbi_call_legacy(SBI_REMOTE_SFENCE_VMA, &hart_mask as *const _ as usize, 0, 0);
}

pub fn remote_sfence_vma_asid(hart_mask: usize, _start: usize, _size: usize, _asid: usize) {
    sbi_call_legacy(
        SBI_REMOTE_SFENCE_VMA_ASID,
        &hart_mask as *const _ as usize,
        0,
        0,
    );
}

const SBI_SET_TIMER: usize = 0;
const SBI_CONSOLE_PUTCHAR: usize = 1;
const SBI_CONSOLE_GETCHAR: usize = 2;
const SBI_CLEAR_IPI: usize = 3;
const SBI_SEND_IPI: usize = 4;
const SBI_REMOTE_FENCE_I: usize = 5;
const SBI_REMOTE_SFENCE_VMA: usize = 6;
const SBI_REMOTE_SFENCE_VMA_ASID: usize = 7;
const SBI_SHUTDOWN: usize = 8;
// Legacy calls end.
