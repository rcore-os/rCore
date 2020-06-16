use alloc::boxed::Box;
use alloc::vec::Vec;
use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct Event: u32 {
        const READABLE                      = 1 << 0;
        const WRITABLE                      = 1 << 1;
        const ERROR                         = 1 << 2;
    }
}

pub type EventHandler = Box<dyn Fn(Event) -> bool + Send>;

#[derive(Default)]
pub struct EventBus {
    event: Event,
    callbacks: Vec<EventHandler>,
}

impl EventBus {
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
