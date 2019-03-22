use super::*;
use core::mem::size_of;
use core::sync::atomic::{AtomicI32, Ordering};
use crate::arch::cpu;

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
    proc.vm
        .check_write_array(buf, strings.len() * offset)?;

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
    proc.vm
        .check_write_array(mask, size / size_of::<u32>())?;

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
    process()
        .vm
        .check_write_ptr(uaddr as *mut AtomicI32)?;
    let atomic = unsafe { &mut *(uaddr as *mut AtomicI32) };
    let timeout = if timeout.is_null() {
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
pub fn sys_reboot(magic: u32, magic2: u32, cmd: u32, arg: *const u8) -> SysResult {
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
