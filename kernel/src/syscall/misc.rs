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
