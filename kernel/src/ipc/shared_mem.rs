use crate::sync::Semaphore;
use crate::sync::SpinLock as Mutex;
use alloc::{boxed::Box, collections::BTreeMap, string::String, sync::Arc, sync::Weak, vec::Vec};
use core::cell::UnsafeCell;
use lazy_static::lazy_static;
use spin::RwLock;
use rcore_memory::{VirtAddr, PhysAddr};
use rcore_memory::memory_set::handler::{Shared, SharedGuard};
use crate::memory::{GlobalFrameAlloc, FrameAllocator};

lazy_static! {
    static ref KEY2SHM: RwLock<BTreeMap<usize, Weak<spin::Mutex<SharedGuard<GlobalFrameAlloc>>>>> = RwLock::new(BTreeMap::new());
}


#[derive(Clone)]
pub struct ShmIdentifier {
    pub addr: VirtAddr,
    pub sharedGuard: Arc<spin::Mutex<SharedGuard<GlobalFrameAlloc>>>
}

impl ShmIdentifier {
    pub fn setAddr(&mut self, addr: VirtAddr) {
        self.addr = addr;
    }
    pub fn new_sharedGuard(key: usize, memsize: usize) -> Arc<spin::Mutex<SharedGuard<GlobalFrameAlloc>>> {
        let mut key2shm = KEY2SHM.write();
    
        // found in the map
        if let Some(weak_guard) = key2shm.get(&key) {
            if let Some(guard) = weak_guard.upgrade() {
                return guard;
            }
        }
        let mut sharedGuard = Arc::new(spin::Mutex::new(SharedGuard::new_with_size(GlobalFrameAlloc, memsize)));
        // insert to global map
        key2shm.insert(key, Arc::downgrade(&sharedGuard));
        sharedGuard
    }
}


