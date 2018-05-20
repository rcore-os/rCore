pub fn bytes_sum<T>(p: &T) -> u8 {
	use core::mem::size_of_val;
	let len = size_of_val(p);
	let p = p as *const T as *const u8;
	(0..len).map(|i| unsafe { &*p.offset(i as isize) })
		.fold(0, |a, &b| a.overflowing_add(b).0)
}

/// 
pub trait Checkable {
	fn check(&self) -> bool;
}

/// Scan memory to find the struct
pub unsafe fn find_in_memory<T: Checkable>
	(begin: usize, len: usize, step: usize) -> Option<usize> {

	(begin .. begin + len).step_by(step)
		.find(|&addr| { (&*(addr as *const T)).check() })
}

use core::ops::IndexMut;
use core::fmt::Debug;

/// Get values by 2 diff keys at the same time
pub trait GetMut2<Idx: Debug + Eq> {
    type Output;
    fn get_mut(&mut self, id: Idx) -> &mut Self::Output;
    fn get_mut2(&mut self, id1: Idx, id2: Idx) -> (&mut Self::Output, &mut Self::Output) {
        assert_ne!(id1, id2);
        let self1 = self as *mut Self;
        let self2 = self1;
        let p1 = unsafe { &mut *self1 }.get_mut(id1);
        let p2 = unsafe { &mut *self2 }.get_mut(id2);
        (p1, p2)
    }
}


pub use self::event::EventHub;

mod event {
    use alloc::BinaryHeap;
    use core::cmp::{Ordering, PartialOrd};

    type Time = usize;

    struct Timer<T> {
        time: Time,
        data: T,
    }

    impl<T> PartialEq for Timer<T> {
        fn eq(&self, other: &Self) -> bool {
            self.time.eq(&other.time)
        }
    }

    impl<T> Eq for Timer<T> {}

    impl<T> PartialOrd for Timer<T> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            other.time.partial_cmp(&self.time)
        }
    }

    impl<T> Ord for Timer<T> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.partial_cmp(&other).unwrap()
        }
    }

    pub struct EventHub<T> {
        tick: Time,
        timers: BinaryHeap<Timer<T>>,
    }

    impl<T> EventHub<T> {
        pub fn new() -> Self {
            EventHub {
                tick: 0,
                timers: BinaryHeap::new(),
            }
        }
        pub fn tick(&mut self) {
            self.tick += 1;
        }
        pub fn pop(&mut self) -> Option<T> {
            match self.timers.peek() {
                None => return None,
                Some(timer) if timer.time != self.tick => return None,
                _ => {}
            };
            self.timers.pop().map(|t| t.data)
        }
        pub fn push(&mut self, time_after: Time, data: T) {
            let time = self.tick + time_after;
            self.timers.push(Timer { time, data });
        }
        pub fn get_time(&self) -> Time {
            self.tick
        }
    }
}