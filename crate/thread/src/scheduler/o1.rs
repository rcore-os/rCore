//! O(1) scheduler introduced in Linux 2.6
//!
//! Two queues are maintained, one is active, another is inactive.
//! Take the first task from the active queue to run. When it is empty, swap active and inactive queues.

use super::*;

pub struct O1Scheduler {
    inner: Mutex<O1SchedulerInner>,
}

struct O1SchedulerInner {
    active_queue: usize,
    queues: [Vec<Tid>; 2],
}

impl Scheduler for O1Scheduler {
    fn push(&self, tid: usize) {
        self.inner.lock().push(tid);
    }
    fn pop(&self, _cpu_id: usize) -> Option<usize> {
        self.inner.lock().pop()
    }
    fn tick(&self, current_tid: usize) -> bool {
        self.inner.lock().tick(current_tid)
    }
    fn set_priority(&self, _tid: usize, _priority: u8) {}
}

impl O1Scheduler {
    pub fn new() -> Self {
        let inner = O1SchedulerInner {
            active_queue: 0,
            queues: [Vec::new(), Vec::new()],
        };
        O1Scheduler {
            inner: Mutex::new(inner),
        }
    }
}

impl O1SchedulerInner {
    fn push(&mut self, tid: Tid) {
        let inactive_queue = 1 - self.active_queue;
        self.queues[inactive_queue].push(tid);
        trace!("o1 push {}", tid - 1);
    }

    fn pop(&mut self) -> Option<Tid> {
        let ret = match self.queues[self.active_queue].pop() {
            Some(tid) => return Some(tid),
            None => {
                // active queue is empty, swap 'em
                self.active_queue = 1 - self.active_queue;
                self.queues[self.active_queue].pop()
            }
        };
        trace!("o1 pop {:?}", ret);
        ret
    }

    fn tick(&mut self, _current: Tid) -> bool {
        true
    }
}
