use alloc::{collections::BinaryHeap, vec::Vec};

use log::*;
use spin::Mutex;

pub use self::rr::RRScheduler;
pub use self::stride::StrideScheduler;

mod rr;
mod stride;

type Pid = usize;

/// The scheduler for a ThreadPool
pub trait Scheduler: Sync + 'static {
    /// Push a thread to the back of ready queue.
    fn push(&self, pid: Pid);
    /// Select a thread to run, pop it from the queue.
    fn pop(&self) -> Option<Pid>;
    /// Got a tick from CPU.
    /// Return true if need reschedule.
    fn tick(&self, current_pid: Pid) -> bool;
    /// Set priority of a thread.
    fn set_priority(&self, pid: Pid, priority: u8);
}

fn expand<T: Default + Clone>(vec: &mut Vec<T>, id: usize) {
    let len = vec.len();
    vec.resize(len.max(id + 1), T::default());
}
