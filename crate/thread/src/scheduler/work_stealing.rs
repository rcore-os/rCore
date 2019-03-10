use super::*;
use deque::{self, Stealer, Worker, Stolen};

pub struct WorkStealingScheduler {
    /// The ready queue of each processors
    workers: Vec<Worker<Tid>>,
    /// Stealers to all processors' queue
    stealers: Vec<Stealer<Tid>>,
}

impl WorkStealingScheduler {
    pub fn new(core_num: usize) -> Self {
        let (workers, stealers) = (0..core_num).map(|_| deque::new()).unzip();
        WorkStealingScheduler { workers, stealers }
    }
}

impl Scheduler for WorkStealingScheduler {
    fn push(&self, tid: usize) {
        // TODO: push to random queue?
        // now just push to cpu0
        self.workers[0].push(tid);
        trace!("work-stealing: cpu0 push thread {}", tid);
    }

    fn pop(&self, cpu_id: usize) -> Option<usize> {
        if let Some(tid) = self.workers[cpu_id].pop() {
            trace!("work-stealing: cpu{} pop thread {}", cpu_id, tid);
            return Some(tid);
        }
        let n = self.workers.len();
        for i in 1..n {
            let mut other_id = cpu_id + i;
            if other_id >= n {
                other_id -= n;
            }
            loop {
                match self.stealers[other_id].steal() {
                    Stolen::Abort => {} // retry
                    Stolen::Empty => break,
                    Stolen::Data(tid) => {
                        trace!("work-stealing: cpu{} steal thread {} from cpu{}", cpu_id, tid, other_id);
                        return Some(tid);
                    }
                }
            }
        }
        None
    }

    fn tick(&self, _current_tid: usize) -> bool {
        true
    }

    fn set_priority(&self, _tid: usize, _priority: u8) {}
}
