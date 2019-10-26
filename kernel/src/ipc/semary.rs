use crate::sync::Semaphore;
use crate::sync::SpinLock as Mutex;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::ops::Index;
use spin::RwLock;

/// A System V semaphore set
pub struct SemArray {
    key: usize,
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
    /// Get the semaphore array with `key`.
    /// If not exist, create a new one with `nsems` elements.
    pub fn get_or_create(key: usize, nsems: usize, _flags: usize) -> Arc<Self> {
        let mut key2sem = KEY2SEM.write();

        // found in the map
        if let Some(weak_array) = key2sem.get(&key) {
            if let Some(array) = weak_array.upgrade() {
                return array;
            }
        }
        // not found, create one
        let mut semaphores = Vec::new();
        for i in 0..nsems {
            semaphores.push(Semaphore::new(0));
        }
        // insert to global map
        let array = Arc::new(SemArray {
            key,
            sems: semaphores,
        });
        key2sem.insert(key, Arc::downgrade(&array));
        array
    }
}
