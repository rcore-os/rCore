use spin::Mutex;
use alloc::BinaryHeap;
use arch::interrupt::TrapFrame;
use core::cmp::{Ordering, PartialOrd};

pub fn get_time() -> i32 {
    info!("get_time:");
    EVENT_HUB.get_time() as i32
}

pub fn timer_handler(tf: &TrapFrame, rsp: &mut usize) {
    // Store rsp to global for `schedule`
    *RSP.lock() = Some(*rsp);

    EVENT_HUB.tick();

    // Take rsp from global
    *rsp = RSP.lock().take().unwrap();
}

static RSP: Mutex<Option<usize>> = Mutex::new(None);

fn schedule() {
    info!("Schedule at time {}", EVENT_HUB.get_time());
    use process;
    process::schedule(RSP.lock().as_mut().unwrap());

    // Repeat
    EVENT_HUB.add(100, schedule);
}

lazy_static! {
    static ref EVENT_HUB: EventHub = {
        let e = EventHub::default();
        e.add(100, schedule);
        info!("EventHub: init");
        e
    };
}

type Time = usize;
type TimerHandler = fn();

#[derive(Debug, Eq, PartialEq)]
struct Timer {
    time: Time,
    handler: TimerHandler,
}

impl PartialOrd for Timer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        other.time.partial_cmp(&self.time)
    }
}

impl Ord for Timer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(&other).unwrap()
    }
}

#[derive(Default)]
struct EventHub {
    tick: Mutex<Time>,
    timers: Mutex<BinaryHeap<Timer>>,
}

impl EventHub {
    fn tick(&self) {
        *self.tick.lock() += 1;
        let tick = *self.tick.lock();
        loop {
            match self.timers.lock().peek() {
                None => return,
                Some(timer) if timer.time != tick => return,
                _ => {}
            }
            let timer = self.timers.lock().pop().unwrap();
            (timer.handler)();
        }
    }
    pub fn add(&self, time_after: Time, handler: TimerHandler) {
        let time = self.get_time() + time_after;
        self.timers.lock().push(Timer { time, handler });
    }
    pub fn get_time(&self) -> Time {
        *self.tick.lock()
    }
}