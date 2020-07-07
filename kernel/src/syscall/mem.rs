use rcore_fs::vfs::MMapArea;
use rcore_memory::memory_set::handler::{Delay, File, Linear, Shared};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::PAGE_SIZE;

use super::*;
use crate::memory::GlobalFrameAlloc;

impl Syscall<'_> {
    pub fn sys_mmap(
        &mut self,
        addr: usize,
        len: usize,
        prot: usize,
        flags: usize,
        fd: usize,
        offset: usize,
    ) -> SysResult {
        let prot = MmapProt::from_bits_truncate(prot);
        let flags = MmapFlags::from_bits_truncate(flags);
        info!(
            "mmap: addr={:#x}, size={:#x}, prot={:?}, flags={:?}, fd={}, offset={:#x}",
            addr, len, prot, flags, fd as isize, offset
        );

        let mut proc = self.process();
        let mut addr = addr;
        if addr == 0 {
            // although NULL can be a valid address
            // but in C, NULL is regarded as allocation failure
            // so just skip it
            addr = PAGE_SIZE;
        }

        if flags.contains(MmapFlags::FIXED) {
            // we have to map it to addr, so remove the old mapping first
            self.vm().pop_with_split(addr, addr + len);
        } else {
            addr = self.vm().find_free_area(addr, len);
        }

        if flags.contains(MmapFlags::ANONYMOUS) {
            if flags.contains(MmapFlags::SHARED) {
                self.vm().push(
                    addr,
                    addr + len,
                    prot.to_attr(),
                    Shared::new(GlobalFrameAlloc),
                    "mmap_anon_shared",
                );
                return Ok(addr);
            } else {
                self.vm().push(
                    addr,
                    addr + len,
                    prot.to_attr(),
                    Delay::new(GlobalFrameAlloc),
                    "mmap_anon",
                );
                return Ok(addr);
            }
        } else {
            let file_like = proc.get_file_like(fd)?;
            let area = MMapArea {
                start_vaddr: addr,
                end_vaddr: addr + len,
                prot: prot.bits(),
                flags: flags.bits(),
                offset,
            };
            file_like.mmap(area)?;
            Ok(addr)
        }
    }

    pub fn sys_mprotect(&mut self, addr: usize, len: usize, prot: usize) -> SysResult {
        let prot = MmapProt::from_bits_truncate(prot);
        info!(
            "mprotect: addr={:#x}, size={:#x}, prot={:?}",
            addr, len, prot
        );
        let _attr = prot.to_attr();

        // TODO: properly set the attribute of the area
        //        now some mut ptr check is fault
        let vm = self.vm();
        let memory_area = vm
            .iter()
            .find(|area| area.is_overlap_with(addr, addr + len));
        if memory_area.is_none() {
            return Err(SysError::ENOMEM);
        }
        Ok(0)
    }

    pub fn sys_munmap(&mut self, addr: usize, len: usize) -> SysResult {
        info!("munmap addr={:#x}, size={:#x}", addr, len);
        self.vm().pop_with_split(addr, addr + len);
        Ok(0)
    }
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

#[cfg(target_arch = "mips")]
bitflags! {
    pub struct MmapFlags: usize {
        /// Changes are shared.
        const SHARED = 1 << 0;
        /// Changes are private.
        const PRIVATE = 1 << 1;
        /// Place the mapping at the exact address
        const FIXED = 1 << 4;
        /// The mapping is not backed by any file. (non-POSIX)
        const ANONYMOUS = 0x800;
    }
}

#[cfg(not(target_arch = "mips"))]
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
    pub fn to_attr(self) -> MemoryAttr {
        let mut attr = MemoryAttr::default().user();
        if self.contains(MmapProt::EXEC) {
            attr = attr.execute();
        }
        // TODO: see sys_mprotect
        //        if !self.contains(MmapProt::WRITE) { attr = attr.readonly(); }
        attr
    }
}
