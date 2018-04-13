use x86_64::structures::idt::*;

pub extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: &mut ExceptionStackFrame, _error_code: u64)
{
    println!("\nEXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    loop {}
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: &mut ExceptionStackFrame, error_code: PageFaultErrorCode)
{
    println!("\nEXCEPTION: PAGE FAULT\n{:#?}\n{:#?}", stack_frame, error_code);
    loop {}
}