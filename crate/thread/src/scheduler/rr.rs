use super::*;

pub struct RRScheduler {
    inner: Mutex<RRSchedulerInner>,
}

struct RRSchedulerInner {
    max_time_slice: usize,
    infos: Vec<RRProcInfo>,
}

#[derive(Debug, Default, Copy, Clone)]
struct RRProcInfo {
    present: bool,
    rest_slice: usize,
    prev: Tid,
    next: Tid,
}

impl Scheduler for RRScheduler {
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

impl RRScheduler {
    pub fn new(max_time_slice: usize) -> Self {
        let inner = RRSchedulerInner {
            max_time_slice,
            infos: Vec::default(),
        };
        RRScheduler {
            inner: Mutex::new(inner),
        }
    }
}

impl RRSchedulerInner {
    fn push(&mut self, tid: Tid) {
        let tid = tid + 1;
        expand(&mut self.infos, tid);
        {
            let info = &mut self.infos[tid];
            assert!(!info.present);
            info.present = true;
            if info.rest_slice == 0 {
                info.rest_slice = self.max_time_slice;
            }
        }
        self._list_add_before(tid, 0);
        trace!("rr push {}", tid - 1);
    }

    fn pop(&mut self) -> Option<Tid> {
        let ret = match self.infos[0].next {
            0 => None,
            tid => {
                self.infos[tid].present = false;
                self._list_remove(tid);
                Some(tid - 1)
            }
        };
        trace!("rr pop {:?}", ret);
        ret
    }

    fn tick(&mut self, current: Tid) -> bool {
        let current = current + 1;
        expand(&mut self.infos, current);
        assert!(!self.infos[current].present);

        let rest = &mut self.infos[current].rest_slice;
        if *rest > 0 {
            *rest -= 1;
        } else {
            warn!("current process rest_slice = 0, need reschedule")
        }
        *rest == 0
    }
}

impl RRSchedulerInner {
    fn _list_add_before(&mut self, i: Tid, at: Tid) {
        let prev = self.infos[at].prev;
        self.infos[i].next = at;
        self.infos[i].prev = prev;
        self.infos[prev].next = i;
        self.infos[at].prev = i;
    }
    fn _list_add_after(&mut self, i: Tid, at: Tid) {
        let next = self.infos[at].next;
        self._list_add_before(i, next);
    }
    fn _list_remove(&mut self, i: Tid) {
        let next = self.infos[i].next;
        let prev = self.infos[i].prev;
        self.infos[next].prev = prev;
        self.infos[prev].next = next;
        self.infos[i].next = 0;
        self.infos[i].prev = 0;
    }
}
