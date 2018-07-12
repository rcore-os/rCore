use process::PROCESSOR;


pub fn timer() {
    let mut processor = PROCESSOR.try().unwrap().lock();
    processor.tick();
}

pub fn before_return() {
    use process::PROCESSOR;
    if let Some(processor) = PROCESSOR.try() {
        processor.lock().schedule();
    }
}