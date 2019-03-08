use rcore_memory::memory_set::handler::Delay;
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::paging::PageTable;
use rcore_memory::Page;
use rcore_memory::PAGE_SIZE;

use crate::memory::GlobalFrameAlloc;

use super::*;

pub fn sys_mmap(mut addr: usize, mut len: usize, prot: usize, flags: usize, fd: i32, offset: usize) -> SysResult {
    let prot = MmapProt::from_bits_truncate(prot);
    let flags = MmapFlags::from_bits_truncate(flags);
    info!("mmap: addr={:#x}, size={:#x}, prot={:?}, flags={:?}, fd={}, offset={:#x}", addr, len, prot, flags, fd, offset);

    let mut proc = process();
    if addr == 0 {
        // although NULL can be a valid address
        // but in C, NULL is regarded as allocation failure
        // so just skip it
        addr = PAGE_SIZE;
    }

    if flags.contains(MmapFlags::FIXED) {
        // we have to map it to addr, so remove the old mapping first
        proc.memory_set.pop(addr, addr + len);
    } else {
        addr = proc.memory_set.find_free_area(addr, len);
    }

    if flags.contains(MmapFlags::ANONYMOUS) {
        if flags.contains(MmapFlags::SHARED) {
            return Err(SysError::EINVAL);
        }
        proc.memory_set.push(addr, addr + len, prot.to_attr(), Delay::new(GlobalFrameAlloc), "mmap");
        return Ok(addr);
    }
    unimplemented!()
}

pub fn sys_mprotect(addr: usize, len: usize, prot: usize) -> SysResult {
    let prot = MmapProt::from_bits_truncate(prot);
    info!("mprotect: addr={:#x}, size={:#x}, prot={:?}", addr, len, prot);

    let mut proc = process();
    let attr = prot.to_attr();

    // FIXME: properly set the attribute of the area
    //        now some mut ptr check is fault
    let memory_area = proc.memory_set.iter().find(|area| area.contains(addr));
    if memory_area.is_none() {
        return Err(SysError::ENOMEM);
    }
    proc.memory_set.edit(|pt| {
        for page in Page::range_of(addr, addr + len) {
            let entry = pt.get_entry(page.start_address()).expect("failed to get entry");
            attr.apply(entry);
        }
    });
    Ok(0)
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
        /// Place the mapping at the exact address
        const FIXED = 1 << 4;
        /// The mapping is not backed by any file. (non-POSIX)
        const ANONYMOUS = 1 << 5;
    }
}

impl MmapProt {
    fn to_attr(self) -> MemoryAttr {
        let mut attr = MemoryAttr::default().user();
        if self.contains(MmapProt::EXEC) { attr = attr.execute(); }
        if !self.contains(MmapProt::WRITE) { attr = attr.readonly(); }
        attr
    }
}
