use alloc::collections::BinaryHeap;
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