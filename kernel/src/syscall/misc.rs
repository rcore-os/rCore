use super::*;
use core::mem::size_of;

pub fn sys_uname(buf: *mut u8) -> SysResult {
    info!("sched_uname: buf: {:?}", buf);

    let offset = 65;
    let strings = ["rCore", "orz", "0.1.0", "1", "machine", "domain"];
    let proc = process();
    proc.memory_set
        .check_mut_array(buf, strings.len() * offset)?;

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
    proc.memory_set
        .check_mut_array(mask, size / size_of::<u32>())?;

    // we only have 4 cpu at most.
    // so just set it.
    unsafe {
        *mask = 0b1111;
    }
    Ok(0)
}

pub fn sys_sysinfo(sys_info: *mut SysInfo) -> SysResult {
    let proc = process();
    proc.memory_set.check_mut_ptr(sys_info)?;

    let sysinfo = SysInfo::default();
    unsafe {
        *sys_info = sysinfo
    };
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
    mem_unit: u32
}