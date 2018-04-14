use x86_64::structures::idt::*;

pub extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame, _error_code: u64)
{
    println!("\nEXCEPTION: DOUBLE FAULT\n{:#?}\nErrorCode: {:#x}", stack_frame, _error_code);
    loop {}
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut ExceptionStackFrame, error_code: PageFaultErrorCode)
{
    println!("\nEXCEPTION: PAGE FAULT\n{:#?}\n{:#?}", stack_frame, error_code);
    loop {}
}

#[cfg(feature = "use_apic")]
use arch::driver::apic::ack;
#[cfg(not(feature = "use_apic"))]
use arch::driver::pic::ack;

use consts::irq::*;

pub extern "x86-interrupt" fn keyboard_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    println!("\nInterupt: Keyboard \n{:#?}", stack_frame);
    ack(IRQ_KBD);
}

pub extern "x86-interrupt" fn serial_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    println!("\nInterupt: Serial \n{:#?}", stack_frame);
    ack(IRQ_COM1);
}

pub extern "x86-interrupt" fn timer_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    println!("\nInterupt: Timer \n{:#?}", stack_frame);
    ack(IRQ_TIMER);    
}