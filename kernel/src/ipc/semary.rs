use crate::sync::Semaphore;
use crate::sync::SpinLock as Mutex;
use crate::syscall::{SemBuf, SysError, SysResult, TimeSpec};
use alloc::{collections::BTreeMap, sync::Arc, sync::Weak, vec::Vec};
use bitflags::*;
use core::ops::Index;
use spin::RwLock;

bitflags! {
    struct SemGetFlag: usize {
        const CREAT = 1 << 9;
        const EXCLUSIVE = 1 << 10;
        const NO_WAIT = 1 << 11;
    }
}

// structure specifies the access permissions on the semaphore set
// key_t?
#[derive(Clone, Copy)]
pub struct IpcPerm {
    pub key: usize, /* Key supplied to semget(2) */
    pub uid: u32,   /* Effective UID of owner */
    pub gid: u32,   /* Effective GID of owner */
    pub cuid: u32,  /* Effective UID of creator */
    pub cgid: u32,  /* Effective GID of creator */
    pub mode: u16,  /* Permissions */
    pub __seq: u16, /* Sequence number */
}

// semid data structure
// struct semid_ds
#[derive(Clone, Copy)]
pub struct SemidDs {
    pub perm: IpcPerm, /* Ownership and permissions */
    pub otime: usize,  /* Last semop time */
    pub ctime: usize,  /* Last change time */
    pub nsems: usize,  /* number of semaphores in set */
}

/// A System V semaphore set
pub struct SemArray {
    pub semid_ds: Mutex<SemidDs>,
    sems: Vec<Semaphore>,
}

impl Index<usize> for SemArray {
    type Output = Semaphore;
    fn index(&self, idx: usize) -> &Semaphore {
        &self.sems[idx]
    }
}

lazy_static! {
    static ref KEY2SEM: RwLock<BTreeMap<usize, Weak<SemArray>>> = RwLock::new(BTreeMap::new());
}

impl SemArray {
    // remove semaphores
    pub fn remove(&self) {
        for sem in self.sems.iter() {
            sem.remove();
        }
    }

    pub fn otime(&self) {
        self.semid_ds.lock().otime = TimeSpec::get_epoch().sec;
    }

    pub fn ctime(&self) {
        self.semid_ds.lock().ctime = TimeSpec::get_epoch().sec;
    }

    /// for IPC_SET
    /// see man semctl(2)
    pub fn set(&self, new: &SemidDs) {
        let mut lock = self.semid_ds.lock();
        lock.perm.uid = new.perm.uid;
        lock.perm.gid = new.perm.gid;
        lock.perm.mode = new.perm.mode & 0x1ff;
    }

    /// Get the semaphore array with `key`.
    /// If not exist, create a new one with `nsems` elements.
    pub fn get_or_create(
        mut key: usize,
        nsems: usize,
        flags: usize,
    ) -> Result<Arc<Self>, SysError> {
        let mut key2sem = KEY2SEM.write();
        let flag = SemGetFlag::from_bits_truncate(flags);

        if key == 0 {
            // IPC_PRIVATE
            // find an empty key slot
            key = (1usize..).find(|i| key2sem.get(i).is_none()).unwrap();
        } else {
            // check existence
            if let Some(weak_array) = key2sem.get(&key) {
                if let Some(array) = weak_array.upgrade() {
                    if flag.contains(SemGetFlag::CREAT) && flag.contains(SemGetFlag::EXCLUSIVE) {
                        // exclusive
                        return Err(SysError::EEXIST);
                    }
                    return Ok(array);
                }
            }
        }

        // not found, create one
        let mut semaphores = Vec::new();
        for _ in 0..nsems {
            semaphores.push(Semaphore::new(0));
        }

        // insert to global map
        let array = Arc::new(SemArray {
            semid_ds: Mutex::new(SemidDs {
                perm: IpcPerm {
                    key,
                    uid: 0,
                    gid: 0,
                    cuid: 0,
                    cgid: 0,
                    // least significant 9 bits
                    mode: (flags as u16) & 0x1ff,
                    __seq: 0,
                },
                otime: 0,
                ctime: TimeSpec::get_epoch().sec,
                nsems,
            }),
            sems: semaphores,
        });
        key2sem.insert(key, Arc::downgrade(&array));
        Ok(array)
    }
}
