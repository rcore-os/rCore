use super::*;
use deque::{self, Stealer, Worker, Stolen};

pub struct WorkStealingScheduler {
    /// The ready queue of each processors
    workers: Vec<Worker<Pid>>,
    /// Stealers to all processors' queue
    stealers: Vec<Stealer<Pid>>,
}

impl WorkStealingScheduler {
    pub fn new(core_num: usize) -> Self {
        let (workers, stealers) = (0..core_num).map(|_| deque::new()).unzip();
        WorkStealingScheduler { workers, stealers }
    }
}

impl Scheduler for WorkStealingScheduler {
    fn push(&self, pid: usize) {
        // TODO: push to random queue?
        // now just push to cpu0
        self.workers[0].push(pid);
        trace!("work-stealing: cpu0 push thread {}", pid);
    }

    fn pop(&self, cpu_id: usize) -> Option<usize> {
        if let Some(pid) = self.workers[cpu_id].pop() {
            trace!("work-stealing: cpu{} pop thread {}", cpu_id, pid);
            return Some(pid);
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
                    Stolen::Data(pid) => {
                        trace!("work-stealing: cpu{} steal thread {} from cpu{}", cpu_id, pid, other_id);
                        return Some(pid);
                    }
                }
            }
        }
        None
    }

    fn tick(&self, _current_pid: usize) -> bool {
        true
    }

    fn set_priority(&self, _pid: usize, _priority: u8) {}
}
