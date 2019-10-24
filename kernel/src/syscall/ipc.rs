use crate::sync::Semaphore;
use crate::sync::SpinLock as Mutex;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use bitflags::*;
use core::cell::UnsafeCell;
use spin::RwLock;

pub use crate::ipc::*;

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

        let mut proc = self.process();
        let mut semarray_table = &mut proc.semaphores;

        let id = (0..).find(|i| semarray_table.get(i).is_none()).unwrap();
        let mut sem_array = SemArray::get_or_create(key, nsems, flags);
        semarray_table.insert(id, sem_array);
        Ok(id)
    }

    pub fn sys_semop(&self, id: usize, ops: *const SemBuf, num_ops: usize) -> SysResult {
        info!("semop: id: {}", id);
        let ops = unsafe { self.vm().check_read_array(ops, num_ops)? };

        let sem_array = self.process().get_semarray(id);
        for &SemBuf { num, op, flags } in ops.iter() {
            let flags = SemFlags::from_bits_truncate(flags);
            if flags.contains(SemFlags::IPC_NOWAIT) {
                unimplemented!("Semaphore: semop.IPC_NOWAIT");
            }
            let sem = &sem_array[num as usize];

            let _result = match op {
                1 => sem.release(),
                -1 => sem.acquire(),
                _ => unimplemented!("Semaphore: semop.(Not 1/-1)"),
            };
            if flags.contains(SemFlags::SEM_UNDO) {
                let mut proc = self.process();
                let old_val = *proc.semundos.get(&(id, num)).unwrap_or(&0);
                let new_val = old_val - op;
                proc.semundos.insert((id, num), new_val);
            }
        }
        Ok(0)
    }

    pub fn sys_semctl(&self, id: usize, num: usize, cmd: usize, arg: isize) -> SysResult {
        info!("semctl: id: {}, num: {}, cmd: {}", id, num, cmd);
        let mut proc = self.process();
        let sem_array: Arc<SemArray> = proc.get_semarray(id);
        let sem = &sem_array[num as usize];

        const GETVAL: usize = 12;
        const GETALL: usize = 13;
        const SETVAL: usize = 16;
        const SETALL: usize = 17;

        match cmd {
            SETVAL => sem.set(arg),
            _ => unimplemented!("Semaphore: Semctl.(Not setval)"),
        }
        Ok(0)
    }
}

#[repr(C)]
pub struct SemBuf {
    num: i16,
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
        const SEM_UNDO = 0x1000;
    }
}
