use rcore_memory::memory_set::handler::Delay;
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::PAGE_SIZE;

use crate::memory::GlobalFrameAlloc;

use super::*;

pub fn sys_mmap(mut addr: usize, len: usize, prot: usize, flags: usize, fd: i32, offset: usize) -> SysResult {
    let prot = MmapProt::from_bits_truncate(prot);
    let flags = MmapFlags::from_bits_truncate(flags);
    info!("mmap addr={:#x}, size={:#x}, prot={:?}, flags={:?}, fd={}, offset={:#x}", addr, len, prot, flags, fd, offset);

    let mut proc = process();
    if addr == 0 {
        // although NULL can be a valid address
        // but in C, NULL is regarded as allocation failure
        // so just skip it
        addr = PAGE_SIZE;
    }
    addr = proc.memory_set.find_free_area(addr, len);

    if flags.contains(MmapFlags::ANONYMOUS) {
        if flags.contains(MmapFlags::SHARED) {
            return Err(SysError::EINVAL);
        }
        let handler = Delay::new(prot_to_attr(prot), GlobalFrameAlloc);
        proc.memory_set.push(addr, addr + len, handler, "mmap");
        return Ok(addr as isize);
    }
    unimplemented!()
}

pub fn sys_munmap(addr: usize, len: usize) -> SysResult {
    info!("munmap addr={:#x}, size={:#x}", addr, len);
    let mut proc = process();
    proc.memory_set.pop(addr, addr + len);
    Ok(0)
}

bitflags! {
    pub struct MmapProt: usize {
        /// Data cannot be accessed
        const NONE = 0;
        /// Data can be read
        const READ = 1 << 0;
        /// Data can be written
        const WRITE = 1 << 1;
        /// Data can be executed
        const EXEC = 1 << 2;
    }
}

bitflags! {
    pub struct MmapFlags: usize {
        /// Changes are shared.
        const SHARED = 1 << 0;
        /// Changes are private.
        const PRIVATE = 1 << 1;
        /// The mapping is not backed by any file. (non-POSIX)
        const ANONYMOUS = 1 << 5;
    }
}

fn prot_to_attr(prot: MmapProt) -> MemoryAttr {
    let mut attr = MemoryAttr::default().user();
    if prot.contains(MmapProt::EXEC) { attr = attr.execute(); }
    if !prot.contains(MmapProt::WRITE) { attr = attr.readonly(); }
    assert!(prot.contains(MmapProt::READ));
    attr
}