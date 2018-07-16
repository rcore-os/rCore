use alloc::BinaryHeap;

type Pid = usize;
const MAX_PROCESS_NUM: usize = 32;

///
pub trait Scheduler {
    fn insert(&mut self, pid: Pid);
    fn remove(&mut self, pid: Pid);
    fn select(&mut self) -> Option<Pid>;
    fn tick(&mut self, current: Pid) -> bool;   // need reschedule?
    fn set_priority(&mut self, pid: Pid, priority: u8);
}

pub use self::rr::RRScheduler;
pub use self::stride::StrideScheduler;

mod rr {
    use super::*;

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
            debug!("insert {}", pid - 1);
        }

        fn remove(&mut self, pid: Pid) {
            let pid = pid + 1;
            assert!(self.infos[pid].present);
            self.infos[pid].present = false;
            self._list_remove(pid);
            debug!("remove {}", pid - 1);
        }

        fn select(&mut self) -> Option<Pid> {
            let ret = match self.infos[0].next {
                0 => None,
                i => Some(i - 1),
            };
            debug!("select {:?}", ret);
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

        fn set_priority(&mut self, pid: usize, priority: u8) {
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
}

mod stride {
    use super::*;

    pub struct StrideScheduler {
        max_time_slice: usize,
        infos: [StrideProcInfo; MAX_PROCESS_NUM],
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
        fn insert(&mut self, pid: Pid) {
            let info = &mut self.infos[pid];
            assert!(!info.present);
            info.present = true;
            if info.rest_slice == 0 {
                info.rest_slice = self.max_time_slice;
            }
            self.queue.push((-info.stride, pid));
            debug!("insert {}", pid);
        }

        fn remove(&mut self, pid: Pid) {
            let info = &mut self.infos[pid];
            assert!(info.present);
            info.present = false;
            // FIXME: Support removing any element
            assert_eq!(self.queue.pop().unwrap().1, pid, "Can only remove the top");
            debug!("remove {}", pid);
        }

        fn select(&mut self) -> Option<Pid> {
            let ret = self.queue.peek().map(|&(_, pid)| pid);
            if let Some(pid) = ret {
                let old_stride = self.infos[pid].stride;
                self.infos[pid].pass();
                let stride = self.infos[pid].stride;
                debug!("{} stride {:#x} -> {:#x}", pid, old_stride, stride);
            }
            debug!("select {:?}", ret);
            ret
        }

        fn tick(&mut self, current: Pid) -> bool {
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
            debug!("{} priority = {}", pid, priority);
        }
    }

    impl StrideScheduler {
        pub fn new(max_time_slice: usize) -> Self {
            StrideScheduler {
                max_time_slice,
                infos: [StrideProcInfo::default(); MAX_PROCESS_NUM],
                queue: BinaryHeap::new(),
            }
        }
    }
}