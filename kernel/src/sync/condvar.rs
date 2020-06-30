use super::*;
use crate::consts::{INFORM_PER_MSEC, USEC_PER_TICK};
use crate::process::{Process, Thread};
use crate::syscall::TimeSpec;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use alloc::vec::Vec;

pub struct RegisteredProcess {
    proc: Arc<SpinNoIrqLock<Process>>,
    tid: usize,
    epfd: usize,
    fd: usize,
}

#[derive(Default)]
pub struct Condvar {
    wait_queue: SpinNoIrqLock<VecDeque<Arc<Thread>>>,
    pub epoll_queue: SpinNoIrqLock<VecDeque<RegisteredProcess>>,
}

impl Condvar {
    pub fn new() -> Self {
        Condvar::default()
    }

    pub fn wait_queue_len(&self) -> usize {
        self.wait_queue.lock().len()
    }

    /// Park current thread and wait for this condvar to be notified.
    #[deprecated(note = "this may leads to lost wakeup problem. please use `wait` instead.")]
    pub fn _wait(&self) {
        // The condvar might be notified between adding to queue and thread parking.
        // So park current thread before wait queue lock is freed.
        // Avoid racing
        let lock = self.add_to_wait_queue();
        //thread::park_action(move || {
        //drop(lock);
        //});
    }

    fn add_to_wait_queue(&self) -> MutexGuard<VecDeque<Arc<Thread>>, SpinNoIrq> {
        let mut lock = self.wait_queue.lock();
        //lock.push_back(Arc::new(thread::current()));
        return lock;
    }

    /// Wait for condvar until condition() returns Some
    pub fn wait_event<T>(condvar: &Condvar, condition: impl FnMut() -> Option<T>) -> T {
        Self::wait_events(&[condvar], condition)
    }

    /// Wait for condvars until condition() returns Some
    pub fn wait_events<T>(condvars: &[&Condvar], mut condition: impl FnMut() -> Option<T>) -> T {
        //let thread = thread::current();
        //let tid = thread.id();
        let tid = 0;
        //let token = Arc::new(thread);
        for condvar in condvars {
            let mut lock = condvar.wait_queue.lock();
            //lock.push_back(token.clone());
        }
        let mut locks = Vec::with_capacity(condvars.len());
        loop {
            for condvar in condvars {
                let lock = condvar.wait_queue.lock();
                locks.push(lock);
            }
            //thread_manager().sleep(tid, 0);
            locks.clear();

            if let Some(res) = condition() {
                let _ = FlagsGuard::no_irq_region();
                //thread_manager().cancel_sleeping(tid);
                return res;
            } else {
                for condvar in condvars {
                    let mut queue = condvar.wait_queue.lock();
                    //if queue.iter().find(|&t| Arc::ptr_eq(t, &token)).is_none() {
                    //queue.push_front(token.clone());
                    //}
                }
            }
            //thread::yield_now();
        }
    }

    /// Park current thread and wait for this condvar to be notified.
    pub fn wait<'a, T, S>(&self, guard: MutexGuard<'a, T, S>) -> MutexGuard<'a, T, S>
    where
        S: MutexSupport,
    {
        let mutex = guard.mutex;
        //let token = Arc::new(thread::current());
        let mut lock = self.wait_queue.lock();
        //lock.push_back(token.clone());

        //thread::park_action(move || {
        //drop(lock);
        //drop(guard);
        //});
        // let mut lock = self.wait_queue.lock();
        // lock.retain(|t| !Arc::ptr_eq(&t, &token));
        mutex.lock()
    }

    /// Park current thread and wait for this condvar to be notified or timeout.
    pub fn wait_timeout<'a, T, S>(
        &self,
        guard: MutexGuard<'a, T, S>,
        timeout: TimeSpec,
    ) -> Option<MutexGuard<'a, T, S>>
    where
        S: MutexSupport,
    {
        let mutex = guard.mutex;
        //let token = Arc::new(thread::current());
        let mut lock = self.wait_queue.lock();
        //lock.push_back(token.clone());
        drop(lock);
        drop(guard);

        let timeout = core::time::Duration::new(timeout.sec as u64, timeout.nsec as u32);
        let begin = crate::trap::uptime_msec();
        if timeout.as_millis() != 0 {
            //sleep(timeout);
        }
        // let mut lock = self.wait_queue.lock();
        // lock.retain(|t| !Arc::ptr_eq(&t, &token));
        let end = crate::trap::uptime_msec();
        if end - begin >= timeout.as_millis() as usize {
            None
        } else {
            Some(mutex.lock())
        }
    }

    pub fn notify_one(&self) {
        let mut queue = self.wait_queue.lock();
        if let Some(t) = queue.front() {
            self.epoll_callback(t);
            // info!("nofity thread: {}", t.id());
            //t.unpark();
            queue.pop_front();
        }
    }

    pub fn notify_all(&self) {
        let mut queue = self.wait_queue.lock();
        for t in queue.iter() {
            self.epoll_callback(t);
            //t.unpark();
        }
        queue.clear();
    }

    /// Notify up to `n` waiters.
    /// Return the number of waiters that were woken up.
    pub fn notify_n(&self, n: usize) -> usize {
        let mut count = 0;
        let mut queue = self.wait_queue.lock();
        for t in queue.iter() {
            if count >= n {
                break;
            }
            self.epoll_callback(t);
            //t.unpark();
            count += 1;
        }
        for _ in 0..count {
            queue.pop_front();
        }
        count
    }

    pub fn register_epoll_list(
        &self,
        proc: Arc<SpinNoIrqLock<Process>>,
        tid: usize,
        epfd: usize,
        fd: usize,
    ) {
        self.epoll_queue.lock().push_back(RegisteredProcess {
            proc: proc,
            tid: tid,
            epfd: epfd,
            fd: fd,
        });
    }

    pub fn unregister_epoll_list(&self, tid: usize, epfd: usize, fd: usize) -> bool {
        let mut epoll_list = self.epoll_queue.lock();
        for idx in 0..epoll_list.len() {
            if epoll_list[idx].tid == tid
                && epoll_list[idx].epfd == epfd
                && epoll_list[idx].fd == fd
            {
                epoll_list.remove(idx);
                return true;
            }
        }
        return false;
    }

    fn epoll_callback(&self, thread: &Arc<Thread>) {
        let epoll_list = self.epoll_queue.lock();
        for ist in epoll_list.iter() {
            //if thread.id() == ist.tid {
            if true {
                let proc = ist.proc.lock();
                match proc.get_epoll_instance(ist.epfd) {
                    Ok(instacne) => {
                        let mut ready_list = instacne.ready_list.lock();
                        ready_list.insert(ist.fd);
                    }
                    Err(_) => {
                        panic!("epoll instance not exist");
                    }
                }
            }
        }
    }
}
