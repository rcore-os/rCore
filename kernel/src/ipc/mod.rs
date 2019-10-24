mod semary;

pub use self::semary::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;

/// Semaphore table in a process
#[derive(Default)]
pub struct SemProc {
    /// Semaphore arrays
    arrays: BTreeMap<SemId, Arc<SemArray>>,
    /// Undo operations when process terminates
    undos: BTreeMap<(SemId, SemNum), SemOp>,
}

/// Semaphore set identifier (in a process)
type SemId = usize;

/// Semaphore number (in an array)
type SemNum = u16;

/// Semaphore operation value
type SemOp = i16;

impl SemProc {
    /// Insert the `array` and return its ID
    pub fn add(&mut self, array: Arc<SemArray>) -> SemId {
        let id = self.get_free_id();
        self.arrays.insert(id, array);
        id
    }
    /// Get a free ID
    fn get_free_id(&self) -> SemId {
        (0..).find(|i| self.arrays.get(i).is_none()).unwrap()
    }
    /// Get an semaphore set by `id`
    pub fn get(&self, id: SemId) -> Option<Arc<SemArray>> {
        self.arrays.get(&id).map(|a| a.clone())
    }
    /// Add an undo operation
    pub fn add_undo(&mut self, id: SemId, num: SemNum, op: SemOp) {
        let old_val = *self.undos.get(&(id, num)).unwrap_or(&0);
        let new_val = old_val - op;
        self.undos.insert((id, num), new_val);
    }
}

/// Fork the semaphore table. Clear undo info.
impl Clone for SemProc {
    fn clone(&self) -> Self {
        SemProc {
            arrays: self.arrays.clone(),
            undos: BTreeMap::default(),
        }
    }
}

/// Auto perform semaphores undo on drop
impl Drop for SemProc {
    fn drop(&mut self) {
        for (&(id, num), &op) in self.undos.iter() {
            debug!("semundo: id: {}, num: {}, op: {}", id, num, op);
            let sem_array = self.arrays[&id].clone();
            let sem = &sem_array[num as usize];
            match op {
                1 => sem.release(),
                0 => {}
                _ => unimplemented!("Semaphore: semundo.(Not 1)"),
            }
        }
    }
}
