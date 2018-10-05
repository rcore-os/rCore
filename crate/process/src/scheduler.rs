use alloc::{collections::BinaryHeap, vec::Vec};

type Pid = usize;


//  implements of process scheduler
pub trait Scheduler {

    /*
    **  @brief  add a new process
    **  @param  pid: Pid            the pid of the process to add
    **  @retval none
    */
    fn insert(&mut self, pid: Pid);

    /*
    **  @brief  remove a processs from the list
    **  @param  pid: Pid            the pid of the process to remove
    **  @retval none
    */
    fn remove(&mut self, pid: Pid);

    /*
    **  @brief  choose a process to run next
    **  @param  none
    **  @retval Option<Pid>         the pid of the process to run or none
    */
    fn select(&mut self) -> Option<Pid>;

    /*
    **  @brief  when a clock interrupt occurs, update the list and check whether need to reschedule
    **  @param  current: Pid        the pid of the process which is running now
    **  @retval bool                if need to reschedule
    */
    fn tick(&mut self, current: Pid) -> bool;   // need reschedule?

    /*
    **  @brief  set the priority of the process
    **  @param  pid: Pid            the pid of the process to be set
    **          priority: u8        the priority to be set
    **  @retval none
    */
    fn set_priority(&mut self, pid: Pid, priority: u8);
}

pub use self::rr::RRScheduler;
pub use self::stride::StrideScheduler;

//  use round-robin scheduling
mod rr {
    use super::*;

    pub struct RRScheduler {
        max_time_slice: usize,
        infos: Vec<RRProcInfo>,
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
            expand(&mut self.infos, pid);
            {
                let info = &mut self.infos[pid];
                assert!(!info.present);
                info.present = true;
                if info.rest_slice == 0 {
                    info.rest_slice = self.max_time_slice;
                }
            }
            self._list_add_before(pid, 0);
            trace!("rr insert {}", pid - 1);
        }

        fn remove(&mut self, pid: Pid) {
            let pid = pid + 1;
            assert!(self.infos[pid].present);
            self.infos[pid].present = false;
            self._list_remove(pid);
            trace!("rr remove {}", pid - 1);
        }

        fn select(&mut self) -> Option<Pid> {
            let ret = match self.infos[0].next {
                0 => None,
                i => Some(i - 1),
            };
            trace!("rr select {:?}", ret);
            ret
        }

        fn tick(&mut self, current: Pid) -> bool {
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

        fn set_priority(&mut self, pid: usize, priority: u8) {
        }
    }

    impl RRScheduler {
        pub fn new(max_time_slice: usize) -> Self {
            RRScheduler {
                max_time_slice,
                infos: Vec::default(),
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

// use stride scheduling
mod stride {
    use super::*;

    pub struct StrideScheduler {
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
        fn insert(&mut self, pid: Pid) {
            expand(&mut self.infos, pid);
            let info = &mut self.infos[pid];
            assert!(!info.present);
            info.present = true;
            if info.rest_slice == 0 {
                info.rest_slice = self.max_time_slice;
            }
            self.queue.push((-info.stride, pid));
            trace!("stride insert {}", pid);
        }

        fn remove(&mut self, pid: Pid) {
            let info = &mut self.infos[pid];
            assert!(info.present);
            info.present = false;
            // BinaryHeap only support pop the top.
            // So in order to remove an arbitrary element,
            // we have to take all elements into a Vec,
            // then push the rest back.
            let rest: Vec<_> = self.queue.drain().filter(|&p| p.1 != pid).collect();
            use core::iter::FromIterator;
            self.queue = BinaryHeap::from_iter(rest.into_iter());
            trace!("stride remove {}", pid);
        }

        fn select(&mut self) -> Option<Pid> {
            let ret = self.queue.peek().map(|&(_, pid)| pid);
            if let Some(pid) = ret {
                let old_stride = self.infos[pid].stride;
                self.infos[pid].pass();
                let stride = self.infos[pid].stride;
                trace!("stride {} {:#x} -> {:#x}", pid, old_stride, stride);
            }
            trace!("stride select {:?}", ret);
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

    impl StrideScheduler {
        pub fn new(max_time_slice: usize) -> Self {
            StrideScheduler {
                max_time_slice,
                infos: Vec::default(),
                queue: BinaryHeap::default(),
            }
        }
    }
}

fn expand<T: Default + Clone>(vec: &mut Vec<T>, id: usize) {
    let len = vec.len();
    vec.resize(len.max(id + 1), T::default());
}

