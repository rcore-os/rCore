use super::*;
use consts::MAX_PROCESS_NUM;

///
pub trait Scheduler {
    fn insert(&mut self, pid: Pid);
    fn remove(&mut self, pid: Pid);
    fn select(&self) -> Option<Pid>;
    fn tick(&mut self, current: Pid) -> bool;   // need reschedule?
}

pub struct RRScheduler {
    max_time_slice: usize,
    infos: [RRProcInfo; MAX_PROCESS_NUM],
}

#[derive(Debug, Default, Copy, Clone)]
struct RRProcInfo {
    present: bool,
    rest_slice: usize,
    prev: Pid,
    next: Pid,
}

impl Scheduler for RRScheduler {
    fn insert(&mut self, pid: Pid) {
        let pid = pid + 1;
        {
            let info = &mut self.infos[pid];
            assert!(!info.present);
            info.present = true;
            if info.rest_slice == 0 {
                info.rest_slice = self.max_time_slice;
            }
        }
        self._list_add_before(pid, 0);
        trace!("RRScheduler: insert {}", pid - 1);
    }

    fn remove(&mut self, pid: Pid) {
        let pid = pid + 1;
        assert!(self.infos[pid].present);
        self.infos[pid].present = false;
        self._list_remove(pid);
        trace!("RRScheduler: remove {}", pid - 1);
    }

    fn select(&self) -> Option<Pid> {
        let ret = match self.infos[0].next {
            0 => None,
            i => Some(i - 1),
        };
        trace!("RRScheduler: select {:?}", ret);
        ret
    }

    fn tick(&mut self, current: Pid) -> bool {
        let current = current + 1;
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

impl RRScheduler {
    pub fn new(max_time_slice: usize) -> Self {
        RRScheduler {
            max_time_slice,
            infos: [RRProcInfo::default(); MAX_PROCESS_NUM],
        }
    }
    fn _list_add_before(&mut self, i: Pid, at: Pid) {
        let prev = self.infos[at].prev;
        self.infos[i].next = at;
        self.infos[i].prev = prev;
        self.infos[prev].next = i;
        self.infos[at].prev = i;
    }
    fn _list_remove(&mut self, i: Pid) {
        let next = self.infos[i].next;
        let prev = self.infos[i].prev;
        self.infos[next].prev = prev;
        self.infos[prev].next = next;
        self.infos[i].next = 0;
        self.infos[i].prev = 0;
    }
}