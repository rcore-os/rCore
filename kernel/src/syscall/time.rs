//! Syscalls for time

use super::*;

pub fn sys_get_time() -> SysResult {
    unsafe { Ok(crate::trap::TICK as isize) }
}