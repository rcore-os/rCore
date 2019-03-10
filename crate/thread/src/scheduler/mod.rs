use alloc::{collections::BinaryHeap, vec::Vec};

use log::*;
use spin::Mutex;

pub use self::rr::RRScheduler;
pub use self::stride::StrideScheduler;
pub use self::work_stealing::WorkStealingScheduler;

mod rr;
mod stride;
mod work_stealing;

type Tid = usize;

/// The scheduler for a ThreadPool
pub trait Scheduler: 'static {
    /// Push a thread to the back of ready queue.
    fn push(&self, tid: Tid);
    /// Select a thread to run, pop it from the queue.
    fn pop(&self, cpu_id: usize) -> Option<Tid>;
    /// Got a tick from CPU.
    /// Return true if need reschedule.
    fn tick(&self, current_tid: Tid) -> bool;
    /// Set priority of a thread.
    fn set_priority(&self, tid: Tid, priority: u8);
}

fn expand<T: Default + Clone>(vec: &mut Vec<T>, id: usize) {
    let len = vec.len();
    vec.resize(len.max(id + 1), T::default());
}
