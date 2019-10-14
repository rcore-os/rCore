use crate::sync::Mutex;
use crate::sync::Semaphore;
use spin::RwLock;

pub use crate::ipc::SemArray;
pub use crate::ipc::SemBuf;

impl Syscall<'_> {
    pub fn sys_semget(key: i32, nsems: i32, semflg: i32) -> SysResult { // ipc not supported yet
        i32 SEMMSL = 256;
        if (nsems < 0 || nsems > SEMMSL) {
            return Err(SysError::EINVAL);
        }

        let mut semarray_table = self.process.semaphores.write();
        let sem_id = (0..)
            .find(|i| match semarray_table.get(i) {
                Some(p) => false,
                _ => true,
            })
            .unwrap();

        let mut sem_array : Arc<Mutex<SemArray>> = new_semary(key, nsems, semflg);

        semarray_table.insert(sem_id, sem_array);
        OK(sem_id)
    }

    pub fn sys_semop(sem_id: i32, sem_opa: *const SemBuf, num_sem_ops: i32) -> SysResult {
        let mut sem_bufs:Vec<SemBuf> = Vec::new();
        unsafe {
            for i in 0..num_sem_ops {
                sem_bufs.push(*(sem_opa + i));
            }
        }

        let mut semarray_table = self.process.semaphores.write();

        for sembuf: SemBuf in sem_ops {
            let mut wait = true;
            if (sembuf.flg == IPC_NOWAIT) {
                wait = false;
            }
            /*match(sembuf.sem_op) {
                x if x > 0 => {
                    let mut semarray: SemArray = semarray_table.get(sem_id).lock();
                    semarray.sems[sembuf.sem_num].modify(x, wait);
                },
                x if x == 0 => {

                },
                x if x < 0 => {

                }
            }*/
            let mut semarray: SemArray = semarray_table.get(sem_id).lock();
            semarray.sems[sembuf.sem_num].modify(x, wait);
        }
    }

}

bitflags! {
    pub struct SEMFLAGS: usize {
        /// For SemOP
        const IPC_NOWAIT = 0x800;
        const SEM_UNDO = 0x1000;
    }
}