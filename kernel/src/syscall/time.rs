//! Syscalls for time

use super::*;
use crate::arch::consts::USEC_PER_TICK;
use crate::arch::driver::rtc_cmos;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref EPOCH_BASE: u64 = unsafe { rtc_cmos::read_epoch() };
    pub static ref TICK_BASE: u64 = unsafe { crate::trap::TICK as u64 };
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TimeVal {
    sec: u64,
    usec: u64,
}

impl TimeVal {
    pub fn to_msec(&self) -> u64 {
        self.sec * 1000 + self.usec / 1000
    }

    pub fn to_usec(&self) -> u64 {
        self.sec * 1000_000 + self.usec
    }
}

pub fn sys_gettimeofday(tv: *mut TimeVal, tz: *const u8) -> SysResult {
    if tz as usize != 0 {
        return Err(SysError::EINVAL);
    }

    let mut proc = process();
    proc.memory_set.check_mut_ptr(tv)?;

    let tick_base = *TICK_BASE;
    let epoch_base = *EPOCH_BASE;
    let tick = unsafe { crate::trap::TICK as u64 };

    let usec = (tick - tick_base) * USEC_PER_TICK as u64;
    let sec = epoch_base + usec / 1_000_000;
    let timeval = TimeVal {
        sec,
        usec: usec % 1_000_000,
    };
    unsafe {
        *tv = timeval;
    }
    Ok(0)
}

pub fn sys_time(time: *mut u64) -> SysResult {
    let tick_base = *TICK_BASE;
    let epoch_base = *EPOCH_BASE;
    let tick = unsafe { crate::trap::TICK as u64 };

    let usec = (tick - tick_base) * USEC_PER_TICK as u64;
    let sec = epoch_base + usec / 1_000_000;
    if time as usize != 0 {
        let mut proc = process();
        proc.memory_set.check_mut_ptr(time)?;
        unsafe {
            time.write(sec as u64);
        }
    }
    Ok(sec as isize)
}
