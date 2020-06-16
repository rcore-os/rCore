use crate::sync::SpinNoIrqLock as Mutex;
use alloc::boxed::Box;
use alloc::{sync::Arc, vec::Vec};
use bitflags::bitflags;
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

bitflags! {
    #[derive(Default)]
    pub struct Event: u32 {
        const READABLE                      = 1 << 0;
        const WRITABLE                      = 1 << 1;
        const ERROR                         = 1 << 2;

        const PROCESS_QUIT                  = 1 << 10;
        const CHILD_PROCESS_QUIT            = 1 << 11;
    }
}

pub type EventHandler = Box<dyn Fn(Event) -> bool + Send>;

#[derive(Default)]
pub struct EventBus {
    event: Event,
    callbacks: Vec<EventHandler>,
}

impl EventBus {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    pub fn set(&mut self, set: Event) {
        self.change(Event::empty(), set);
    }

    pub fn clear(&mut self, set: Event) {
        self.change(set, Event::empty());
    }

    pub fn change(&mut self, reset: Event, set: Event) {
        let orig = self.event;
        let mut new = self.event;
        new.remove(reset);
        new.insert(set);
        self.event = new;
        if new != orig {
            self.callbacks.retain(|f| !f(new));
        }
    }

    pub fn subscribe(&mut self, callback: EventHandler) {
        self.callbacks.push(callback);
    }
}

pub fn wait_for_event(bus: Arc<Mutex<EventBus>>, mask: Event) -> impl Future<Output = Event> {
    EventBusFuture { bus, mask }
}

struct EventBusFuture {
    bus: Arc<Mutex<EventBus>>,
    mask: Event,
}

impl Future for EventBusFuture {
    type Output = Event;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut lock = self.bus.lock();
        if !(lock.event & self.mask).is_empty() {
            return Poll::Ready(lock.event);
        }
        let waker = cx.waker().clone();
        let mask = self.mask;
        lock.subscribe(Box::new(move |s| {
            if (s & mask).is_empty() {
                return false;
            }
            waker.wake_by_ref();
            true
        }));
        Poll::Pending
    }
}
