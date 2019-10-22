
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use crate::sync::SpinLock as Mutex;
use crate::sync::Semaphore;
use spin::RwLock;
use bitflags::*;
use core::cell::UnsafeCell;

pub use crate::ipc::SemArray;
pub use crate::ipc::SemBuf;
pub use crate::ipc::new_semary;
pub use crate::ipc::SemctlUnion;
pub use crate::ipc::semary::SemArrTrait;

use super::*;

impl Syscall<'_> {
    pub fn sys_semget(&self, key: usize, nsems: usize, semflg: usize) -> SysResult {
        info!("sys_semget: key: {}", key);
        let SEMMSL: usize = 256;
        if (nsems < 0 || nsems > SEMMSL) {
            return Err(SysError::EINVAL);
        }

        let mut proc = self.process();
        let mut semarray_table/*: &mut BTreeMap<usize, Arc<SemArray>>*/ = &mut proc.semaphores;

        let sem_id = (0..)
            .find(|i| match semarray_table.get(i) {
                Some(p) => false,
                _ => true,
            })
            .unwrap();

        let mut sem_array : Arc<SemArray> = new_semary(key, nsems, semflg);

        semarray_table.insert(sem_id, sem_array);
        Ok(sem_id)
    }

    pub fn sys_semop(&self, sem_id: usize, sem_ops: *const SemBuf, num_sem_ops: usize) -> SysResult {
        info!("sys_semop: sem_id: {}", sem_id);
        //let mut sem_bufs:Vec<SemBuf> = Vec::new();
        let sem_ops = unsafe { self.vm().check_read_array(sem_ops, num_sem_ops)? };

        //let mut semarray_table/*: &BTreeMap<usize, Arc<SemArray>>*/ = proc.semaphores;

        for sembuf in sem_ops.iter() {
            if (sembuf.sem_flg == (SEMFLAGS::IPC_NOWAIT.bits())) {
                unimplemented!("Semaphore: semop.IPC_NOWAIT");
            }
            //let mut semarray_arc: Arc<SemArray> = (*((*semarray_table).get(&sem_id).unwrap())).clone();
            //let mut semarray: &mut SemArray = &mut *semarray_arc;
            
            let sem_array;
            {
                let mut proc = self.process();
                sem_array = proc.get_semarray(sem_id);
            }
            let sem_ptr = sem_array.get_x(sembuf.sem_num as usize);
            
            let mut result;

            match(sembuf.sem_op) {
                1 => result = sem_ptr.release(),
                -1 => result = sem_ptr.acquire(),
                _ => unimplemented!("Semaphore: semop.(Not 1/-1)"),
            }
            if (sembuf.sem_flg == (SEMFLAGS::SEM_UNDO.bits())) {
                let mut proc = self.process();
                let get_key = proc.semundos.get(&(sem_id, sembuf.sem_num));
                let mut val = 0;
                if (!get_key.is_none()) {
                    val = *get_key.unwrap();
                }
                val -= sembuf.sem_op;
                proc.semundos.insert((sem_id, sembuf.sem_num), val);
                //unimplemented!("Semaphore: semop.SEM_UNDO");
            }
        }
        info!("sem_op: {}", sem_ops[0].sem_op);
        Ok(0)
    }

    pub fn sys_semctl(&self, sem_id: usize, sem_num: usize, cmd: usize, arg: isize) -> SysResult {
        info!("sys_semctl: sem_id: {}", sem_id);
        let mut proc = self.process();
        let sem_array: Arc<SemArray> = proc.get_semarray(sem_id);
        let sem_ptr =  sem_array.get_x(sem_num as usize);
        //let mut sem_ptr: &mut Semaphore = proc.get_semarray(sem_id).get_x(sem_num as usize);

        if (cmd == SEMCTLCMD::SETVAL.bits()) {
            match (sem_ptr.set(arg)) {
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