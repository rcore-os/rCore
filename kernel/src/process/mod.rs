use alloc::String;
use memory::MemorySet;
use self::process::*;
pub use self::processor::*;
use spin::Once;
use sync::SpinNoIrqLock;

mod process;
mod processor;
mod scheduler;


pub fn init() {
    PROCESSOR.call_once(|| {
        SpinNoIrqLock::new({
            let initproc = Process::new_init();
            let idleproc = Process::new("idle", idle_thread, 0);
            let mut processor = Processor::new();
            processor.add(initproc);
            processor.add(idleproc);
            processor
        })
    });
}

pub static PROCESSOR: Once<SpinNoIrqLock<Processor>> = Once::new();

extern fn idle_thread(_arg: usize) -> ! {
    println!("Hello, I'm idle.");
    loop {}
}

pub fn add_user_process(name: impl AsRef<str>, data: &[u8]) {
    let mut processor = PROCESSOR.try().unwrap().lock();
    let mut new = Process::new_user(data);
    new.name = String::from(name.as_ref());
    processor.add(new);
}

pub fn add_kernel_process(entry: extern fn(usize) -> !, arg: usize) -> Pid {
    let mut processor = PROCESSOR.try().unwrap().lock();
    let new = Process::new("", entry, arg);
    processor.add(new)
}

pub fn print() {
    debug!("{:#x?}", *PROCESSOR.try().unwrap().lock());
}