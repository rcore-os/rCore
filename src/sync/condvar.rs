use thread;
use alloc::VecDeque;
use super::SpinNoIrqLock;

pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<thread::Thread>>,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar { wait_queue: SpinNoIrqLock::new(VecDeque::new()) }
    }
    pub fn wait(&self) {
        self.wait_queue.lock().push_back(thread::current());
        thread::park();
    }
    pub fn notify_one(&self) {
        if let Some(t) = self.wait_queue.lock().pop_front() {
            t.unpark();
        }
    }
    pub fn notify_all(&self) {
        while let Some(t) = self.wait_queue.lock().pop_front() {
            t.unpark();
        }
    }
}