#![allow(dead_code)]

use bitflags::*;

pub use crate::ipc::*;

use crate::memory::GlobalFrameAlloc;
use rcore_memory::memory_set::handler::{Shared, SharedGuard};
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::{PhysAddr, VirtAddr, PAGE_SIZE};

use super::*;

impl Syscall<'_> {
    pub fn sys_semget(&self, key: usize, nsems: isize, flags: usize) -> SysResult {
        info!("semget: key: {}", key);

        /// The maximum semaphores per semaphore set
        const SEMMSL: usize = 256;

        if nsems < 0 || nsems as usize > SEMMSL {
            return Err(SysError::EINVAL);
        }
        let nsems = nsems as usize;

        let sem_array = SemArray::get_or_create(key, nsems, flags);
        let id = self.process().semaphores.add(sem_array);
        Ok(id)
    }

    pub fn sys_semop(&self, id: usize, ops: *const SemBuf, num_ops: usize) -> SysResult {
        info!("semop: id: {}", id);
        let ops = unsafe { self.vm().check_read_array(ops, num_ops)? };

        let sem_array = self.process().semaphores.get(id).ok_or(SysError::EINVAL)?;
        sem_array.otime();
        for &SemBuf { num, op, flags } in ops.iter() {
            let flags = SemFlags::from_bits_truncate(flags);
            if flags.contains(SemFlags::IPC_NOWAIT) {
                unimplemented!("Semaphore: semop.IPC_NOWAIT");
            }
            let sem = &sem_array[num as usize];

            let _result = match op {
                1 => sem.release(),
                -1 => sem.acquire()?,
                _ => unimplemented!("Semaphore: semop.(Not 1/-1)"),
            };
            sem.set_pid(self.process().pid.get());
            if flags.contains(SemFlags::SEM_UNDO) {
                self.process().semaphores.add_undo(id, num, op);
            }
        }
        Ok(0)
    }

    pub fn sys_semctl(&self, id: usize, num: usize, cmd: usize, arg: isize) -> SysResult {
        info!("semctl: id: {}, num: {}, cmd: {}", id, num, cmd);
        let sem_array = self.process().semaphores.get(id).ok_or(SysError::EINVAL)?;
        const IPC_RMID: usize = 0;
        const IPC_SET: usize = 1;
        const IPC_STAT: usize = 2;
        const GETPID: usize = 11;
        const GETVAL: usize = 12;
        const GETALL: usize = 13;
        const GETNCNT: usize = 14;
        const GETZCNT: usize = 15;
        const SETVAL: usize = 16;
        const SETALL: usize = 17;

        match cmd {
            IPC_RMID => {
                sem_array.remove();
                Ok(0)
            }
            IPC_SET => {
                // TODO: update IpcPerm
                sem_array.ctime();
                Ok(0)
            }
            IPC_STAT => {
                *unsafe { self.vm().check_write_ptr(arg as *mut SemidDs)? } =
                    *sem_array.semid_ds.lock();
                Ok(0)
            }
            _ => {
                let sem = &sem_array[num as usize];
                match cmd {
                    GETPID => Ok(sem.get_pid()),
                    GETVAL => Ok(sem.get() as usize),
                    GETNCNT => Ok(sem.get_ncnt()),
                    GETZCNT => Ok(0),
                    SETVAL => {
                        sem.set(arg);
                        sem.set_pid(self.process().pid.get());
                        sem_array.ctime();
                        Ok(0)
                    }
                    _ => unimplemented!("Semaphore Semctl cmd: {}", cmd),
                }
            }
        }
    }

    pub fn sys_shmget(&self, key: usize, size: usize, shmflg: usize) -> SysResult {
        info!("shmget: key: {}", key);

        let shared_guard = ShmIdentifier::new_shared_guard(key, size);
        let id = self.process().shm_identifiers.add(shared_guard);
        Ok(id)
    }

    pub fn sys_shmat(&self, id: usize, mut addr: VirtAddr, shmflg: usize) -> SysResult {
        let mut shm_identifier = self
            .process()
            .shm_identifiers
            .get(id)
            .ok_or(SysError::EINVAL)?;

        let mut proc = self.process();
        if addr == 0 {
            // although NULL can be a valid address
            // but in C, NULL is regarded as allocation failure
            // so just skip it
            addr = PAGE_SIZE;
        }
        let size = shm_identifier.shared_guard.lock().size;
        info!("shmat: id: {}, addr = {:#x}, size = {}", id, addr, size);
        addr = self.vm().find_free_area(addr, size);
        self.vm().push(
            addr,
            addr + size,
            MemoryAttr::default().user().execute().writable(),
            Shared::new_with_guard(GlobalFrameAlloc, shm_identifier.shared_guard.clone()),
            "shmat",
        );
        shm_identifier.addr = addr;
        proc.shm_identifiers.set(id, shm_identifier);
        //self.process().shmIdentifiers.setVirtAddr(id, addr);
        return Ok(addr);
    }

    pub fn sys_shmdt(&self, id: usize, addr: VirtAddr, shmflg: usize) -> SysResult {
        info!("shmdt: addr={:#x}", addr);
        let mut proc = self.process();
        let opt_id = proc.shm_identifiers.get_id(addr);
        if let Some(id) = opt_id {
            proc.shm_identifiers.pop(id);
        }
        Ok(0)
    }
}

/// An operation to be performed on a single semaphore
///
/// Ref: [http://man7.org/linux/man-pages/man2/semop.2.html]
#[repr(C)]
pub struct SemBuf {
    num: u16,
    op: i16,
    flags: i16,
}

pub union SemctlUnion {
    val: isize,
    buf: usize,   // semid_ds*, unimplemented
    array: usize, // short*, unimplemented
} // unused

bitflags! {
    pub struct SemFlags: i16 {
        /// For SemOP
        const IPC_NOWAIT = 0x800;
        /// it will be automatically undone when the process terminates.
        const SEM_UNDO = 0x1000;
    }
}
