#[macro_use]
#[path = "./template.rs"]
mod template;
use self::template::*;
pub type TrapFrame = InterruptStackP;

interrupt_stack!(breakpoint, stack, {
    println!("\nEXCEPTION: Breakpoint");
    stack.dump();
});

interrupt_error_p!(double_fault, stack, {
    println!("\nEXCEPTION: Double Fault");
    stack.dump();
    loop {}
});

interrupt_error_p!(page_fault, stack, {
    use x86_64::registers::control_regs::cr2;
    println!("\nEXCEPTION: Page Fault\nAddress: {:#x}", cr2());
    stack.dump();
    loop {}
});

interrupt_error_p!(general_protection_fault, stack, {
    println!("\nEXCEPTION: General Protection Fault");
    stack.dump();
    loop {}
});

interrupt_stack_p!(invalid_opcode, stack, {
    println!("\nEXCEPTION: Invalid Opcode");
    stack.dump();
    loop {}
});

#[cfg(feature = "use_apic")]
use arch::driver::apic::ack;
#[cfg(not(feature = "use_apic"))]
use arch::driver::pic::ack;

use super::consts::*;

interrupt!(keyboard, {
    use arch::driver::keyboard;
    println!("\nInterupt: Keyboard");
    let c = keyboard::get();
    println!("Key = '{}' {}", c as u8 as char, c);
    ack(IRQ_KBD);

});

interrupt!(com1, {
    use arch::driver::serial::COM1;
    println!("\nInterupt: COM1");
    COM1.lock().receive();
    ack(IRQ_COM1);
});

interrupt!(com2, {
    use arch::driver::serial::COM2;
    println!("\nInterupt: COM2");
    COM2.lock().receive();
    ack(IRQ_COM2);
});

use spin::Mutex;
// FIXME: Deadlock
//static TICK: Mutex<usize> = Mutex::new(0);

interrupt_switch!(timer, rsp, {
//    let mut tick = TICK.lock();
//    *tick += 1;
//    let tick = *tick;
    static mut tick: usize = 0;
    unsafe{ tick += 1; }
    if tick % 100 == 0 {
        println!("\nInterupt: Timer\ntick = {}", tick);
        use process;
        process::schedule(rsp);
    }
    ack(IRQ_TIMER);
});

interrupt_stack_p!(to_user, stack, {
    use arch::gdt;
    println!("\nInterupt: To User");
    let rsp = unsafe{ (stack as *const InterruptStackP).offset(1) } as usize;
    gdt::set_ring0_rsp(rsp);
    stack.iret.cs = gdt::UCODE_SELECTOR.0 as usize;
    stack.iret.ss = gdt::UDATA_SELECTOR.0 as usize;
    stack.iret.rflags |= 3 << 12;   // 设置EFLAG的I/O特权位，使得在用户态可使用in/out指令
});

interrupt_stack_p!(to_kernel, stack, {
//    println!("rsp @ {:#x}", stack as *const _ as usize);
    use arch::gdt;
    println!("\nInterupt: To Kernel");
    stack.iret.cs = gdt::KCODE_SELECTOR.0 as usize;
    stack.iret.ss = gdt::KDATA_SELECTOR.0 as usize;
});

interrupt_stack_p!(syscall, stack, {
    println!("\nInterupt: Syscall");
});