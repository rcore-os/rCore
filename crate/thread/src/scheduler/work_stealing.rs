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
        // not random, but uniform
        // no sync, because we don't need to
        static mut WORKER_CPU: usize = 0;
        let n = self.workers.len();
        let mut cpu = unsafe {
            WORKER_CPU = WORKER_CPU + 1;
            if WORKER_CPU >= n {
                WORKER_CPU -= n;
            }
            WORKER_CPU
        };

        // potential racing, so we just check once more
        if cpu >= n {
            cpu -= n;
        }
        self.workers[cpu].push(tid);
        trace!("work-stealing: cpu{} push thread {}", cpu, tid);
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
