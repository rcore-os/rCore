use super::*;
use crate::arch::cpu;
use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};
use crate::consts::USER_STACK_SIZE;

pub fn sys_arch_prctl(code: i32, addr: usize, tf: &mut TrapFrame) -> SysResult {
    const ARCH_SET_FS: i32 = 0x1002;
    match code {
        #[cfg(target_arch = "x86_64")]
        ARCH_SET_FS => {
            info!("sys_arch_prctl: set FS to {:#x}", addr);
            tf.fsbase = addr;
            Ok(0)
        }
        _ => Err(SysError::EINVAL),
    }
}

pub fn sys_uname(buf: *mut u8) -> SysResult {
    info!("sched_uname: buf: {:?}", buf);

    let offset = 65;
    let strings = ["rCore", "orz", "0.1.0", "1", "machine", "domain"];
    let proc = process();
    proc.vm.check_write_array(buf, strings.len() * offset)?;

    for i in 0..strings.len() {
        unsafe {
            util::write_cstr(buf.add(i * offset), &strings[i]);
        }
    }
    Ok(0)
}

pub fn sys_sched_getaffinity(pid: usize, size: usize, mask: *mut u32) -> SysResult {
    info!(
        "sched_getaffinity: pid: {}, size: {}, mask: {:?}",
        pid, size, mask
    );
    let proc = process();
    proc.vm.check_write_array(mask, size / size_of::<u32>())?;

    // we only have 4 cpu at most.
    // so just set it.
    unsafe {
        *mask = 0b1111;
    }
    Ok(0)
}

pub fn sys_sysinfo(sys_info: *mut SysInfo) -> SysResult {
    let proc = process();
    proc.vm.check_write_ptr(sys_info)?;

    let sysinfo = SysInfo::default();
    unsafe { *sys_info = sysinfo };
    Ok(0)
}

pub fn sys_futex(uaddr: usize, op: u32, val: i32, timeout: *const TimeSpec) -> SysResult {
    info!(
        "futex: [{}] uaddr: {:#x}, op: {:#x}, val: {}, timeout_ptr: {:?}",
        thread::current().id(),
        uaddr,
        op,
        val,
        timeout
    );
    //    if op & OP_PRIVATE == 0 {
    //        unimplemented!("futex only support process-private");
    //        return Err(SysError::ENOSYS);
    //    }
    if uaddr % size_of::<u32>() != 0 {
        return Err(SysError::EINVAL);
    }
    process().vm.check_write_ptr(uaddr as *mut AtomicI32)?;
    let atomic = unsafe { &mut *(uaddr as *mut AtomicI32) };
    let _timeout = if timeout.is_null() {
        None
    } else {
        process().vm.check_read_ptr(timeout)?;
        Some(unsafe { *timeout })
    };

    const OP_WAIT: u32 = 0;
    const OP_WAKE: u32 = 1;
    const OP_PRIVATE: u32 = 128;

    let queue = process().get_futex(uaddr);

    match op & 0xf {
        OP_WAIT => {
            if atomic.load(Ordering::Acquire) != val {
                return Err(SysError::EAGAIN);
            }
            // FIXME: support timeout
            queue._wait();
            Ok(0)
        }
        OP_WAKE => {
            let woken_up_count = queue.notify_n(val as usize);
            Ok(woken_up_count)
        }
        _ => {
            warn!("unsupported futex operation: {}", op);
            Err(SysError::ENOSYS)
        }
    }
}

const LINUX_REBOOT_CMD_HALT: u32 = 0xcdef0123;
pub fn sys_reboot(_magic: u32, magic2: u32, cmd: u32, _arg: *const u8) -> SysResult {
    // we will skip verifying magic
    if cmd == LINUX_REBOOT_CMD_HALT {
        unsafe {
            cpu::exit_in_qemu(1);
        }
    }
    Ok(0)
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct SysInfo {
    uptime: u64,
    loads: [u64; 3],
    totalram: u64,
    freeram: u64,
    sharedram: u64,
    bufferram: u64,
    totalswap: u64,
    freeswap: u64,
    procs: u16,
    totalhigh: u64,
    freehigh: u64,
    mem_unit: u32,
}

const RLIMIT_STACK: usize = 3;
const RLIMIT_RSS: usize = 5;
const RLIMIT_NOFILE: usize = 7;
const RLIMIT_AS: usize = 9;

pub fn sys_prlimit64(
    pid: usize,
    resource: usize,
    new_limit: *const RLimit,
    old_limit: *mut RLimit,
) -> SysResult {
    let proc = process();
    info!(
        "prlimit64: pid: {}, resource: {}, new_limit: {:x?}, old_limit: {:x?}",
        pid, resource, new_limit, old_limit
    );
    match resource {
        RLIMIT_STACK => {
            if !old_limit.is_null() {
                proc.vm.check_write_ptr(old_limit)?;
                unsafe {
                    *old_limit = RLimit {
                        cur: USER_STACK_SIZE as u64,
                        max: USER_STACK_SIZE as u64,
                    };
                }
            }
            Ok(0)
        }
        RLIMIT_NOFILE => {
            if !old_limit.is_null() {
                proc.vm.check_write_ptr(old_limit)?;
                unsafe {
                    *old_limit = RLimit {
                        cur: 1024,
                        max: 1024,
                    };
                }
            }
            Ok(0)
        },
        RLIMIT_RSS | RLIMIT_AS => {
            if !old_limit.is_null() {
                proc.vm.check_write_ptr(old_limit)?;
                unsafe {
                    // 1GB
                    *old_limit = RLimit {
                        cur: 1024 * 1024 * 1024,
                        max: 1024 * 1024 * 1024,
                    };
                }
            }
            Ok(0)
        }
        _ => Err(SysError::ENOSYS),
    }
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct RLimit {
    cur: u64, // soft limit
    max: u64, // hard limit
}
