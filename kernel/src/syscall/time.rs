//! Syscalls for time

use super::*;
use crate::consts::USEC_PER_TICK;
use core::time::Duration;
use lazy_static::lazy_static;
use rcore_fs::vfs::Timespec;

impl Syscall<'_> {
    pub fn sys_gettimeofday(
        &mut self,
        mut tv: UserOutPtr<TimeVal>,
        tz: UserInPtr<u8>,
    ) -> SysResult {
        info!("gettimeofday: tv: {:?}, tz: {:?}", tv, tz);
        // don't support tz
        if !tz.is_null() {
            return Err(SysError::EINVAL);
        }

        let timeval = TimeVal::get_epoch();
        tv.write(timeval)?;
        Ok(0)
    }

    pub fn sys_clock_gettime(&mut self, clock: usize, mut ts: UserOutPtr<TimeSpec>) -> SysResult {
        info!("clock_gettime: clock: {:?}, ts: {:?}", clock, ts);

        let timespec = TimeSpec::get_epoch();
        ts.write(timespec)?;
        Ok(0)
    }

    #[cfg(target_arch = "x86_64")]
    pub fn sys_time(&mut self, time: *mut u64) -> SysResult {
        let sec = get_epoch_usec() / USEC_PER_SEC;
        if time as usize != 0 {
            let time = unsafe { self.vm().check_write_ptr(time)? };
            *time = sec as u64;
        }
        Ok(sec as usize)
    }

    pub fn sys_getrusage(&mut self, who: usize, rusage: *mut RUsage) -> SysResult {
        info!("getrusage: who: {}, rusage: {:?}", who, rusage);
        let rusage = unsafe { self.vm().check_write_ptr(rusage)? };

        let tick_base = *TICK_BASE;
        let tick = unsafe { crate::trap::wall_tick() as u64 };

        let usec = (tick - tick_base) * USEC_PER_TICK as u64;
        let new_rusage = RUsage {
            utime: TimeVal {
                sec: (usec / USEC_PER_SEC) as usize,
                usec: (usec % USEC_PER_SEC) as usize,
            },
            stime: TimeVal {
                sec: (usec / USEC_PER_SEC) as usize,
                usec: (usec % USEC_PER_SEC) as usize,
            },
        };
        *rusage = new_rusage;
        Ok(0)
    }

    pub fn sys_times(&mut self, buf: *mut Tms) -> SysResult {
        info!("times: buf: {:?}", buf);
        let buf = unsafe { self.vm().check_write_ptr(buf)? };

        let _tick_base = *TICK_BASE;
        let tick = unsafe { crate::trap::wall_tick() as u64 };

        let new_buf = Tms {
            tms_utime: 0,
            tms_stime: 0,
            tms_cutime: 0,
            tms_cstime: 0,
        };

        *buf = new_buf;
        Ok(tick as usize)
    }
}

// should be initialized together
lazy_static! {
    pub static ref EPOCH_BASE: u64 = crate::drivers::rtc::read_epoch();
    pub static ref TICK_BASE: u64 = unsafe { crate::trap::wall_tick() as u64 };
}

// 1ms msec
// 1us usec
// 1ns nsec

const USEC_PER_SEC: u64 = 1_000_000;
const MSEC_PER_SEC: u64 = 1_000;
const USEC_PER_MSEC: u64 = 1_000;
const NSEC_PER_USEC: u64 = 1_000;
const NSEC_PER_MSEC: u64 = 1_000_000;

/// Get time since epoch in usec
fn get_epoch_usec() -> u64 {
    let tick_base = *TICK_BASE;
    let epoch_base = *EPOCH_BASE;
    let tick = unsafe { crate::trap::wall_tick() as u64 };

    (tick - tick_base) * USEC_PER_TICK as u64 + epoch_base * USEC_PER_SEC
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TimeVal {
    sec: usize,
    usec: usize,
}

impl TimeVal {
    pub fn to_msec(&self) -> u64 {
        (self.sec as u64) * MSEC_PER_SEC + (self.usec as u64) / USEC_PER_MSEC
    }

    pub fn get_epoch() -> Self {
        let usec = get_epoch_usec();
        TimeVal {
            sec: (usec / USEC_PER_SEC) as usize,
            usec: (usec % USEC_PER_SEC) as usize,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TimeSpec {
    pub sec: usize,
    pub nsec: usize,
}

impl TimeSpec {
    pub fn to_msec(&self) -> u64 {
        (self.sec as u64) * MSEC_PER_SEC + (self.nsec as u64) / NSEC_PER_MSEC
    }

    pub fn to_duration(&self) -> Duration {
        Duration::new(self.sec as u64, self.nsec as u32)
    }

    pub fn get_epoch() -> Self {
        let usec = get_epoch_usec();
        TimeSpec {
            sec: (usec / USEC_PER_SEC) as usize,
            nsec: (usec % USEC_PER_SEC * NSEC_PER_USEC) as usize,
        }
    }

    // TODO: more precise; update when write
    pub fn update(inode: &Arc<dyn INode>) {
        let now = TimeSpec::get_epoch().into();
        if let Ok(mut metadata) = inode.metadata() {
            metadata.atime = now;
            metadata.mtime = now;
            metadata.ctime = now;
            // silently fail for device file
            inode.set_metadata(&metadata).ok();
        }
    }

    pub fn is_zero(&self) -> bool {
        self.sec == 0 && self.nsec == 0
    }
}

impl Into<Timespec> for TimeSpec {
    fn into(self) -> Timespec {
        Timespec {
            sec: self.sec as i64,
            nsec: self.nsec as i32,
        }
    }
}

// ignore other fields for now
#[repr(C)]
pub struct RUsage {
    utime: TimeVal,
    stime: TimeVal,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Tms {
    tms_utime: u64,  /* user time */
    tms_stime: u64,  /* system time */
    tms_cutime: u64, /* user time of children */
    tms_cstime: u64, /* system time of children */
}
