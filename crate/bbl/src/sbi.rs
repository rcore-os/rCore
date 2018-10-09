//! Port from sbi.h
//! 
//! This code is used for OS to use hardware outside with calling these implements

/*
**  @brief  translate implement calling message to RISCV asm
**  @param  which: usize                ecall type
**          arg0, arg1, arg2: usize     ecall args
**  @retval ret: usize                  the result of the asm
*/
#[inline(always)]
fn sbi_call(which: usize, arg0: usize, arg1: usize, arg2: usize) -> usize {
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

/*
**  @brief  output char to console
**  @param  ch: usize       the char to output to console
**  @retval none
*/
pub fn console_putchar(ch: usize) {
    sbi_call(SBI_CONSOLE_PUTCHAR, ch, 0, 0);
}

/*
**  @brief  input char from console
**  @param  none
**  @retval ch: usize       the char get from console
*/
pub fn console_getchar() -> usize {
    sbi_call(SBI_CONSOLE_GETCHAR, 0, 0, 0)
}

/*
**  @brief  call this function to shutdown
**  @param  none
**  @retval none
*/
pub fn shutdown() {
    sbi_call(SBI_SHUTDOWN, 0, 0, 0);
}

/*
**  @brief  set a timer when running
**  @param  stime_value: u64    time to be set
**  @retval none
*/
pub fn set_timer(stime_value: u64) {
    #[cfg(target_pointer_width = "32")]
    sbi_call(SBI_SET_TIMER, stime_value as usize, (stime_value >> 32) as usize, 0);
    #[cfg(target_pointer_width = "64")]
    sbi_call(SBI_SET_TIMER, stime_value as usize, 0, 0);
}

/*
**  @brief  clear the ipi
**  @param  none
**  @retval none
*/
pub fn clear_ipi() {
    sbi_call(SBI_CLEAR_IPI, 0, 0, 0);
}

/*
**  @brief  
**  @param  
**  @retval none
*/
pub fn send_ipi(hart_mask: *const usize) {
    sbi_call(SBI_SEND_IPI, hart_mask as usize, 0, 0);
}

/*
**  @brief  
**  @param  
**  @retval none
*/
pub fn remote_fence_i(hart_mask: *const usize) {
    sbi_call(SBI_REMOTE_FENCE_I, hart_mask as usize, 0, 0);
}

/*
**  @brief  
**  @param  
**  @retval none
*/
pub fn remote_sfence_vma(hart_mask: *const usize, _start: usize, _size: usize) {
    sbi_call(SBI_REMOTE_SFENCE_VMA, hart_mask as usize, 0, 0);
}

/*
**  @brief  
**  @param  
**  @retval none
*/
pub fn remote_sfence_vma_asid(hart_mask: *const usize, _start: usize, _size: usize, _asid: usize) {
    sbi_call(SBI_REMOTE_SFENCE_VMA_ASID, hart_mask as usize, 0, 0);
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
