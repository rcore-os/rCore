use lazy_static::lazy_static;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use crate::sync::SpinNoIrqLock as Mutex;
use crate::sync::Semaphore;
use spin::RwLock;

pub struct SemArray {
    pub key: usize,
    pub sems: Vec<Semaphore>
}

impl SemArray {
    pub fn new(key: usize, sems: Vec<Semaphore>) -> SemArray {
        SemArray {
            key,
            sems
        }
    }
}

pub struct SemBuf {
    pub sem_num: i16,
    pub sem_op: i16,
    pub sem_flg: i16,
}

pub struct SemUndo {
    pub sem_id: i16,
    pub sem_num: i16,
    pub sem_op: i16
}

pub union SemctlUnion {
    pub val: isize,
    pub buf: usize, // semid_ds*, unimplemented
    pub array: usize, // short*, unimplemented
} // unused

lazy_static! {
    pub static ref KEY2SEM: RwLock<BTreeMap<usize, Weak<Mutex<SemArray>>>> =
        RwLock::new(BTreeMap::new());                                                   // not mentioned.
}

pub fn new_semary(key: usize, nsems: usize, semflg: usize) -> Arc<Mutex<SemArray>> {
    let mut key2sem_table = KEY2SEM.write();
    let mut sem_array_ref: Arc<Mutex<SemArray>>;

    if key2sem_table.get(&key).is_none() {
        let mut semaphores: Vec<Semaphore> = Vec::new();
        for i in 0..nsems {
            semaphores.push(Semaphore::new(0));
        }

        let mut sem_array = SemArray::new(key, semaphores);
        sem_array_ref = Arc::new(Mutex::new(sem_array));
        key2sem_table.insert(key, Arc::downgrade(&sem_array_ref));
    } else {
        sem_array_ref = key2sem_table.get(&key).unwrap().upgrade().unwrap();                               // no security check
    }

    sem_array_ref
}