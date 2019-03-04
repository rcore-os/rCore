//! Syscalls for time

use super::*;
use crate::arch::driver::rtc_cmos;

pub fn sys_gettimeofday(tv: *mut u64, tz: *const u8) -> SysResult {
    if tz as usize != 0 {
        return Err(SysError::EINVAL);
    }

    let mut proc = process();
    proc.memory_set.check_mut_ptr(tv)?;

    unsafe { *tv = rtc_cmos::read_epoch() };
    Ok(0)
}

pub fn sys_time(time: *mut u64) -> SysResult {
    let t = rtc_cmos::read_epoch();
    if time as usize != 0 {
        let mut proc = process();
        proc.memory_set.check_mut_ptr(time)?;
        unsafe {
            time.write(t as u64);
        }
    }
    Ok(t as isize)
}
