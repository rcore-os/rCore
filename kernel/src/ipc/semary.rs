use crate::sync::Semaphore;
use crate::sync::SpinLock as Mutex;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::cell::UnsafeCell;
use lazy_static::lazy_static;
use spin::RwLock;

pub trait SemArrTrait {
    fn get_x(&self, x: usize) -> &Semaphore;
}

pub struct SemArray {
    pub key: usize,
    pub sems: Vec<Semaphore>,
}

unsafe impl Sync for SemArray {}
unsafe impl Send for SemArray {}

impl SemArray {
    pub fn new(key: usize, sems: Vec<Semaphore>) -> SemArray {
        SemArray {
            key: key,
            sems: sems,
        }
    }
}

impl SemArrTrait for SemArray {
    fn get_x(&self, x: usize) -> &Semaphore {
        &self.sems[x]
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
    pub sem_op: i16,
}

pub union SemctlUnion {
    pub val: isize,
    pub buf: usize,   // semid_ds*, unimplemented
    pub array: usize, // short*, unimplemented
} // unused

lazy_static! {
    pub static ref KEY2SEM: RwLock<BTreeMap<usize, Weak<SemArray>>> =
        RwLock::new(BTreeMap::new());                                                   // not mentioned.
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
}
