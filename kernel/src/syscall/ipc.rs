
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use crate::sync::SpinNoIrqLock as Mutex;
use crate::sync::Semaphore;
use spin::RwLock;
use bitflags::*;

pub use crate::ipc::SemArray;
pub use crate::ipc::SemBuf;
pub use crate::ipc::new_semary;
pub use crate::ipc::SemctlUnion;

use super::*;

impl Syscall<'_> {
    pub fn sys_semget(&self, key: usize, nsems: usize, semflg: usize) -> SysResult { // ipc not supported yet
        let SEMMSL: usize = 256;
        if (nsems < 0 || nsems > SEMMSL) {
            return Err(SysError::EINVAL);
        }

        let mut proc = self.process();
        let mut semarray_table = proc.semaphores.write();
        let sem_id = (0..)
            .find(|i| match semarray_table.get(i) {
                Some(p) => false,
                _ => true,
            })
            .unwrap();

        let mut sem_array : Arc<Mutex<SemArray>> = new_semary(key, nsems, semflg);

        semarray_table.insert(sem_id, sem_array);
        Ok(sem_id)
    }

    pub fn sys_semop(&self, sem_id: usize, sem_ops: *const SemBuf, num_sem_ops: usize) -> SysResult {
        //let mut sem_bufs:Vec<SemBuf> = Vec::new();
        let sem_ops = unsafe { self.vm().check_read_array(sem_ops, num_sem_ops)? };

        let mut proc = self.process();
        let mut semarray_table = proc.semaphores.write();

        for sembuf in sem_ops.iter() {
            if (sembuf.sem_flg == (SEMFLAGS::IPC_NOWAIT.bits())) {
                unimplemented!("Semaphore: semop.IPC_NOWAIT");
            }
            if (sembuf.sem_flg == (SEMFLAGS::SEM_UNDO.bits())) {
                unimplemented!("Semaphore: semop.SEM_UNDO");
            }
            let mut semarray_arc: Arc<Mutex<SemArray>> = (*semarray_table.get(&sem_id).unwrap()).clone();
            let mut semarray: &SemArray = &*semarray_arc.lock();
            match((*semarray).sems[sembuf.sem_num as usize].modify(sembuf.sem_op as isize, true)) {
                Ok(0) => {},
                Err(()) => {
                    return Err(SysError::EAGAIN);
                },
                _ => {
                    return Err(SysError::EUNDEF);                                                                      // unknown error?
                }
            }
            /*if (sembuf.sem_flg == (SEMFLAGS::SEM_UNDO.bits())) {
                proc.semops.push(sembuf);
            }*/
        }
        Ok(0)
    }

    pub fn sys_semctl(&self, sem_id: usize, sem_num: usize, cmd: usize, arg: isize) -> SysResult {
        let mut proc = self.process();
        let mut semarray_table = proc.semaphores.write();
        let mut semarray_arc: Arc<Mutex<SemArray>> = (*semarray_table.get(&sem_id).unwrap()).clone();
        let mut semarray: &SemArray = &*semarray_arc.lock();
        if (cmd == SEMCTLCMD::SETVAL.bits()) {
            match (*semarray).sems[sem_num].set(arg) {
                Ok(()) => {
                    return Ok(0);
                }
                _ => {
                    return Err(SysError::EUNDEF);
                }
            }
            
        } else {
            unimplemented!("Semaphore: Semctl.(Not setval)");
        }
    }
}

bitflags! {
    pub struct SEMFLAGS: i16 {
        /// For SemOP
        const IPC_NOWAIT = 0x800;
        const SEM_UNDO = 0x1000;
    }
}

bitflags! {
    pub struct SEMCTLCMD: usize {
        //const GETVAL = 12;
        //const GETALL = 13;
        const SETVAL = 16;
        //const SETALL = 17;
    }
}