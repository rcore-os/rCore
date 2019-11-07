use crate::sync::Semaphore;
use crate::sync::SpinLock as Mutex;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::cell::UnsafeCell;
use lazy_static::lazy_static;
use spin::RwLock;
use rcore_memory::{VirtAddr, PhysAddr};

pub struct shmid {
    key: usize,
    size: usize,
    target: PhysAddr
}

pub struct shmid_local {
    key: usize,
    size: usize,
    addr: VirtAddr,
    target: PhysAddr
}

impl shmid {
    pub fn new(key: usize, size: usize, target: PhysAddr) -> shmid {
        shmid {
            key,
            size,
            target
        }
    }
}

impl shmid_local {
    pub fn new(key: usize, size: usize, addr: VirtAddr, target: PhysAddr) -> shmid {
        shmid_local {
            key,
            size,
            addr,
            target
        }
    } 
}

lazy_static! {
    pub static ref KEY2SHM: RwLock<BTreeMap<usize, Arc<shmid>>> =
        RwLock::new(BTreeMap::new());                                                   // between ARC & WEAK
}

/*pub fn new_shm(key: usize, size: usize, shmflg: usize) -> shmid_local {
    let mut key2shm_table = KEY2SHM.write();
    let mut shmid_ref: shmid;
    let mut key_shmid_ref = key2shm_table.get(&key);
    if (key_shmid_ref.is_none() || key_shmid_ref.unwrap().upgrade().is_none()) {
        proc.
    } else {
        shmid_ref = key2shm_table.get(&key).unwrap().unwrap();

    }

    shmid_ref
}

pub fn new_semary(key: usize, nsems: usize, semflg: usize) -> Arc<SemArray> {
    let mut key2sem_table = KEY2SEM.write();
    let mut sem_array_ref: Arc<SemArray>;

    let mut key_sem_array_ref = key2sem_table.get(&key);
    if (key_sem_array_ref.is_none() || key_sem_array_ref.unwrap().upgrade().is_none()) {
        let mut semaphores: Vec<Semaphore> = Vec::new();
        for i in 0..nsems {
            semaphores.push(Semaphore::new(0));
        }

        let mut sem_array = SemArray::new(key, semaphores);
        sem_array_ref = Arc::new(sem_array);
        key2sem_table.insert(key, Arc::downgrade(&sem_array_ref));
    } else {
        sem_array_ref = key2sem_table.get(&key).unwrap().upgrade().unwrap(); // no security check
    }

    sem_array_ref
}*/
