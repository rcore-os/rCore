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
    use x86_64::registers::control_regs::cr2;
    println!("\nEXCEPTION: PAGE FAULT\n{:#?}\nErrorCode: {:#?}\nAddress: {:#x}",
             stack_frame, error_code, cr2());
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
    use arch::driver::keyboard;
    println!("\nInterupt: Keyboard");
    let c = keyboard::get();
    println!("Key = '{}' {}", c as u8 as char, c);
    ack(IRQ_KBD);
}

pub extern "x86-interrupt" fn com1_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    use arch::driver::serial::COM1;
    println!("\nInterupt: COM1");
    COM1.lock().receive();
    ack(IRQ_COM1);
}

pub extern "x86-interrupt" fn com2_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    use arch::driver::serial::COM2;
    println!("\nInterupt: COM2");
    COM2.lock().receive();
    ack(IRQ_COM2);
}

use spin::Mutex;
static TICK: Mutex<usize> = Mutex::new(0);

pub extern "x86-interrupt" fn timer_handler(
    stack_frame: &mut ExceptionStackFrame)
{
    let mut tick = TICK.lock();
    *tick += 1;
    let tick = *tick;
    if tick % 100 == 0 {
        println!("\nInterupt: Timer\ntick = {}", tick);
    }
    ack(IRQ_TIMER);    
}