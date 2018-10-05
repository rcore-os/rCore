use process::*;
use arch::interrupt::TrapFrame;

/*
* @brief:
*   process timer interrupt
*/
pub fn timer() {
    let mut processor = processor();
    processor.tick();
}

pub fn before_return() {
    if let Some(processor) = PROCESSOR.try() {
        processor.lock().schedule();
    }
}

/*
* @param: 
*   TrapFrame: the error's trapframe
* @brief: 
*   process the error trap, if processor inited then exit else panic!
*/
pub fn error(tf: &TrapFrame) -> ! {
    if let Some(processor) = PROCESSOR.try() {
        let mut processor = processor.lock();
        let pid = processor.current_pid();
        error!("Process {} error:\n{:#x?}", pid, tf);
        processor.exit(pid, 0x100); // TODO: Exit code for error
        processor.schedule();
        unreachable!();
    } else {
        panic!("Exception when processor not inited\n{:#x?}", tf);
    }
}