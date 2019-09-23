use rcore_memory::memory_set::handler::{Delay, File, Linear};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::PAGE_SIZE;

use crate::memory::GlobalFrameAlloc;

use super::*;

impl Syscall<'_> {
    pub fn sys_mmap(
        &mut self,
        mut addr: usize,
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
            addr, len, prot, flags, fd, offset
        );

        let mut proc = self.process();
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
                return Err(SysError::EINVAL);
            }
            self.vm().push(
                addr,
                addr + len,
                prot.to_attr(),
                Delay::new(GlobalFrameAlloc),
                "mmap_anon",
            );
            return Ok(addr);
        } else {
            let file = proc.get_file(fd)?;
            info!("mmap path is {} ", &*file.path);
            match &*file.path {
                "/dev/fb0" => {
                    use crate::drivers::gpu::fb::FRAME_BUFFER;
                    let attr = prot.to_attr();
                    #[cfg(feature = "board_raspi3")]
                    let attr = attr.mmio(crate::arch::paging::MMIOType::NormalNonCacheable as u8);

                    if let Some(fb) = FRAME_BUFFER.lock().as_ref() {
                        self.vm().push(
                            addr,
                            addr + len,
                            attr,
                            Linear::new((fb.paddr() - addr) as isize),
                            "mmap_file",
                        );
                        info!("mmap for /dev/fb0");
                        return Ok(addr);
                    } else {
                        return Err(SysError::ENOENT);
                    }
                }
                _ => {
                    let inode = file.inode();
                    self.vm().push(
                        addr,
                        addr + len,
                        prot.to_attr(),
                        File {
                            file: INodeForMap(inode),
                            mem_start: addr,
                            file_start: offset,
                            file_end: offset + len,
                            allocator: GlobalFrameAlloc,
                        },
                        "mmap_file",
                    );
                    return Ok(addr);
                }
            };
        }
    }

    pub fn sys_mprotect(&mut self, addr: usize, len: usize, prot: usize) -> SysResult {
        let prot = MmapProt::from_bits_truncate(prot);
        info!(
            "mprotect: addr={:#x}, size={:#x}, prot={:?}",
            addr, len, prot
        );
        let attr = prot.to_attr();

        // FIXME: properly set the attribute of the area
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
    fn to_attr(self) -> MemoryAttr {
        let mut attr = MemoryAttr::default().user();
        if self.contains(MmapProt::EXEC) {
            attr = attr.execute();
        }
        // FIXME: see sys_mprotect
        //        if !self.contains(MmapProt::WRITE) { attr = attr.readonly(); }
        attr
    }
}
