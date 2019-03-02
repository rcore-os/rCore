//! Syscalls for time

use super::*;

pub fn sys_get_time() -> SysResult {
    unsafe { Ok(crate::trap::TICK as isize) }
}

pub fn sys_time(time: *mut u64) -> SysResult {
    let t = unsafe { crate::trap::TICK };
    if time as usize != 0 {
        let mut proc = process();
        if !proc.memory_set.check_mut_ptr(time) {
            return Err(SysError::EFAULT);
        }
        unsafe {
            time.write(t as u64);
        }
    }
    Ok(t as isize)
}
