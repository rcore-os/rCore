//! A simple timer

use alloc::collections::VecDeque;

type Time = usize;

struct Event<T> {
    time: Time,
    data: T,
}

/// A simple timer using ordered dequeue
pub struct Timer<T> {
    tick: Time,
    timers: VecDeque<Event<T>>,
}

impl<T: PartialEq> Timer<T> {
    /// Create a new timer.
    pub fn new() -> Self {
        Timer {
            tick: 0,
            timers: VecDeque::new(),
        }
    }
    /// Called on each tick.
    pub fn tick(&mut self) {
        self.tick += 1;
    }
    /// Pop an expired timer after `tick`.
    ///
    /// This must be called after calling `tick`,
    /// and should be called multiple times until return `None`.
    pub fn pop(&mut self) -> Option<T> {
        match self.timers.front() {
            None => return None,
            Some(timer) if timer.time != self.tick => return None,
            _ => {}
        };
        self.timers.pop_front().map(|t| t.data)
    }
    /// Start a timer with given time interval
    pub fn start(&mut self, time_after: Time, data: T) {
        //debug!("{:?} {:?}", self.tick, time_after);
        let time = self.tick + time_after;
        let event = Event { time, data };
        let mut it = self.timers.iter();
        let mut i: usize = 0;
        loop {
            match it.next() {
                None => break,
                Some(e) if e.time >= time => break,
                _ => {}
            }
            i += 1;
        }
        self.timers.insert(i, event);
    }
    /// Stop a timer
    pub fn stop(&mut self, data: T) {
        if let Some(i) = self.timers.iter().position(|t| t.data == data) {
            self.timers.remove(i);
        }
    }
}
