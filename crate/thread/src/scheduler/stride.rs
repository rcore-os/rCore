use super::*;

pub struct StrideScheduler {
    inner: Mutex<StrideSchedulerInner>,
}

pub struct StrideSchedulerInner {
    max_time_slice: usize,
    infos: Vec<StrideProcInfo>,
    queue: BinaryHeap<(Stride, Pid)>, // It's max heap, so pass < 0
}

#[derive(Debug, Default, Copy, Clone)]
struct StrideProcInfo {
    present: bool,
    rest_slice: usize,
    stride: Stride,
    priority: u8,
}

impl StrideProcInfo {
    fn pass(&mut self) {
        const BIG_STRIDE: Stride = 1 << 20;
        let pass = if self.priority == 0 {
            BIG_STRIDE
        } else {
            BIG_STRIDE / self.priority as Stride
        };
        // FIXME: overflowing_add is not working ???
        // self.stride.overflowing_add(pass);
        self.stride += pass;
    }
}

type Stride = i32;

impl Scheduler for StrideScheduler {
    fn push(&self, pid: usize) {
        self.inner.lock().push(pid);
    }
    fn pop(&self) -> Option<usize> {
        self.inner.lock().pop()
    }
    fn tick(&self, current_pid: usize) -> bool {
        self.inner.lock().tick(current_pid)
    }
    fn set_priority(&self, pid: usize, priority: u8) {
        self.inner.lock().set_priority(pid, priority);
    }
}

impl StrideScheduler {
    pub fn new(max_time_slice: usize) -> Self {
        let inner = StrideSchedulerInner {
            max_time_slice,
            infos: Vec::default(),
            queue: BinaryHeap::default(),
        };
        StrideScheduler { inner: Mutex::new(inner) }
    }
}

impl StrideSchedulerInner {
    fn push(&mut self, pid: Pid) {
        expand(&mut self.infos, pid);
        let info = &mut self.infos[pid];
        assert!(!info.present);
        info.present = true;
        if info.rest_slice == 0 {
            info.rest_slice = self.max_time_slice;
        }
        self.queue.push((-info.stride, pid));
        trace!("stride push {}", pid);
    }

    fn pop(&mut self) -> Option<Pid> {
        let ret = self.queue.pop().map(|(_, pid)| pid);
        if let Some(pid) = ret {
            let old_stride = self.infos[pid].stride;
            self.infos[pid].pass();
            let stride = self.infos[pid].stride;
            trace!("stride {} {:#x} -> {:#x}", pid, old_stride, stride);
        }
        trace!("stride pop {:?}", ret);
        ret
    }

    fn tick(&mut self, current: Pid) -> bool {
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

    fn set_priority(&mut self, pid: Pid, priority: u8) {
        self.infos[pid].priority = priority;
        trace!("stride {} priority = {}", pid, priority);
    }
}
