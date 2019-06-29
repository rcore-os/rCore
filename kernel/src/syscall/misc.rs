use super::*;
use crate::arch::cpu;
use crate::consts::USER_STACK_SIZE;
use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};

impl Syscall<'_> {
    #[cfg(target_arch = "x86_64")]
    pub fn sys_arch_prctl(&mut self, code: i32, addr: usize) -> SysResult {
        const ARCH_SET_FS: i32 = 0x1002;
        match code {
            ARCH_SET_FS => {
                info!("sys_arch_prctl: set FSBASE to {:#x}", addr);
                self.tf.fsbase = addr;
                Ok(0)
            }
            _ => Err(SysError::EINVAL),
        }
    }

    pub fn sys_uname(&mut self, buf: *mut u8) -> SysResult {
        info!("uname: buf: {:?}", buf);

        let offset = 65;
        let strings = ["rCore", "orz", "0.1.0", "1", "machine", "domain"];
        let buf = unsafe { self.vm().check_write_array(buf, strings.len() * offset)? };

        for i in 0..strings.len() {
            unsafe {
                util::write_cstr(&mut buf[i * offset], &strings[i]);
            }
        }
        Ok(0)
    }

    pub fn sys_sched_getaffinity(&mut self, pid: usize, size: usize, mask: *mut u32) -> SysResult {
        info!(
            "sched_getaffinity: pid: {}, size: {}, mask: {:?}",
            pid, size, mask
        );
        let mask = unsafe { self.vm().check_write_array(mask, size / size_of::<u32>())? };

        // we only have 4 cpu at most.
        // so just set it.
        mask[0] = 0b1111;
        Ok(0)
    }

    pub fn sys_sysinfo(&mut self, sys_info: *mut SysInfo) -> SysResult {
        let sys_info = unsafe { self.vm().check_write_ptr(sys_info)? };

        let sysinfo = SysInfo::default();
        *sys_info = sysinfo;
        Ok(0)
    }

    pub fn sys_futex(
        &mut self,
        uaddr: usize,
        op: u32,
        val: i32,
        timeout: *const TimeSpec,
    ) -> SysResult {
        info!(
            "futex: [{}] uaddr: {:#x}, op: {:#x}, val: {}, timeout_ptr: {:?}",
            thread::current().id(),
            uaddr,
            op,
            val,
            timeout
        );
        if op & OP_PRIVATE == 0 {
            warn!("process-shared futex is unimplemented");
        }
        if uaddr % size_of::<u32>() != 0 {
            return Err(SysError::EINVAL);
        }
        let atomic = unsafe { self.vm().check_write_ptr(uaddr as *mut AtomicI32)? };

        const OP_WAIT: u32 = 0;
        const OP_WAKE: u32 = 1;
        const OP_PRIVATE: u32 = 0x80;

        let mut proc = self.process();
        let queue = proc.get_futex(uaddr);

        match op & 0xf {
            OP_WAIT => {
                let _timeout = if timeout.is_null() {
                    None
                } else {
                    Some(unsafe { *self.vm().check_read_ptr(timeout)? })
                };

                if atomic.load(Ordering::Acquire) != val {
                    return Err(SysError::EAGAIN);
                }
                // FIXME: support timeout
                queue.wait(proc);
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

    pub fn sys_reboot(
        &mut self,
        _magic: u32,
        _magic2: u32,
        cmd: u32,
        _arg: *const u8,
    ) -> SysResult {
        // we will skip verifying magic
        if cmd == LINUX_REBOOT_CMD_HALT {
            unsafe {
                cpu::exit_in_qemu(1);
            }
        } else if cmd == LINUX_REBOOT_CMD_RESTART {
            unsafe {
                cpu::reboot();
            }
        }
        Ok(0)
    }

    pub fn sys_prlimit64(
        &mut self,
        pid: usize,
        resource: usize,
        new_limit: *const RLimit,
        old_limit: *mut RLimit,
    ) -> SysResult {
        info!(
            "prlimit64: pid: {}, resource: {}, new_limit: {:x?}, old_limit: {:x?}",
            pid, resource, new_limit, old_limit
        );
        match resource {
            RLIMIT_STACK => {
                if !old_limit.is_null() {
                    let old_limit = unsafe { self.vm().check_write_ptr(old_limit)? };
                    *old_limit = RLimit {
                        cur: USER_STACK_SIZE as u64,
                        max: USER_STACK_SIZE as u64,
                    };
                }
                Ok(0)
            }
            RLIMIT_NOFILE => {
                if !old_limit.is_null() {
                    let old_limit = unsafe { self.vm().check_write_ptr(old_limit)? };
                    *old_limit = RLimit {
                        cur: 1024,
                        max: 1024,
                    };
                }
                Ok(0)
            }
            RLIMIT_RSS | RLIMIT_AS => {
                if !old_limit.is_null() {
                    let old_limit = unsafe { self.vm().check_write_ptr(old_limit)? };
                    // 1GB
                    *old_limit = RLimit {
                        cur: 1024 * 1024 * 1024,
                        max: 1024 * 1024 * 1024,
                    };
                }
                Ok(0)
            }
            _ => Err(SysError::ENOSYS),
        }
    }

    pub fn sys_getrandom(&mut self, buf: *mut u8, len: usize, _flag: u32) -> SysResult {
        //info!("getrandom: buf: {:?}, len: {:?}, falg {:?}", buf, len,flag);
        let slice = unsafe { self.vm().check_write_array(buf, len)? };
        let mut i = 0;
        for elm in slice {
            unsafe {
                *elm = i + crate::trap::TICK as u8;
            }
            i += 1;
        }

        Ok(len)
    }
}

const LINUX_REBOOT_CMD_RESTART: u32 = 0x01234567;
const LINUX_REBOOT_CMD_HALT: u32 = 0xCDEF0123;
const LINUX_REBOOT_CMD_CAD_ON: u32 = 0x89ABCDEF;
const LINUX_REBOOT_CMD_CAD_OFF: u32 = 0x00000000;
const LINUX_REBOOT_CMD_POWER_OFF: u32 = 0x4321FEDC;
const LINUX_REBOOT_CMD_RESTART2: u32 = 0xA1B2C3D4;
const LINUX_REBOOT_CMD_SW_SUSPEND: u32 = 0xD000FCE2;
const LINUX_REBOOT_CMD_KEXEC: u32 = 0x45584543;

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

#[repr(C)]
#[derive(Debug, Default)]
pub struct RLimit {
    cur: u64, // soft limit
    max: u64, // hard limit
}
