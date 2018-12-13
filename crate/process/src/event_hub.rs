use alloc::collections::VecDeque;
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
    timers: VecDeque<Timer<T>>,
}

impl<T: PartialEq> EventHub<T> {
    pub fn new() -> Self {
        EventHub {
            tick: 0,
            timers: VecDeque::new(),
        }
    }
    pub fn tick(&mut self) {
        self.tick += 1;
    }
    pub fn pop(&mut self) -> Option<T> {
        match self.timers.front() {
            None => return None,
            Some(timer) if timer.time != self.tick => return None,
            _ => {}
        };
        self.timers.pop_front().map(|t| t.data)
    }
    pub fn push(&mut self, time_after: Time, data: T) {
        //debug!("{:?} {:?}", self.tick, time_after);
        let time = self.tick + time_after;
        let timer = Timer { time, data };
        let mut it = self.timers.iter();
        let mut i : usize = 0;
        loop {
            let now = it.next();
            if now == None {
                break
            };
            if now.unwrap() < &timer {
                break
            };
            i += 1;
        }
        self.timers.insert(i, timer);
    }
    pub fn get_time(&self) -> Time {
        self.tick
    }
    pub fn remove(&mut self, data: T) {
        let mut it = self.timers.iter();
        let mut i : usize = 0;
        loop {
            let now = it.next();
            if now == None {
                break
            };
            if now.map(|t| &t.data).unwrap() == &data {
                break
            };
            i += 1;
        }
        if i < self.timers.len() {
            self.timers.remove(i);
        }
    }
}