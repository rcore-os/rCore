#[macro_use]
#[path = "./template.rs"]
mod template;
use self::template::*;
pub type TrapFrame = InterruptStackP;

interrupt_stack!(breakpoint, stack, {
    error!("\nEXCEPTION: Breakpoint");
    stack.dump();
});

interrupt_error_p!(double_fault, stack, {
    error!("\nEXCEPTION: Double Fault");
    stack.dump();
    loop {}
});

interrupt_error_p!(page_fault, stack, {
    use x86_64::registers::control_regs::cr2;
    let addr = cr2().0;
    error!("\nEXCEPTION: Page Fault @ {:#x}, code: {:#x}", addr, stack.code);

    use memory::page_fault_handler;
    if page_fault_handler(addr) {
        return;
    }

    stack.dump();
    loop {}
});

interrupt_error_p!(general_protection_fault, stack, {
    error!("\nEXCEPTION: General Protection Fault");
    stack.dump();
    loop {}
});

interrupt_stack_p!(invalid_opcode, stack, {
    error!("\nEXCEPTION: Invalid Opcode");
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
    info!("\nInterupt: Keyboard");
    let c = keyboard::get();
    info!("Key = '{}' {}", c as u8 as char, c);
    ack(IRQ_KBD);

});

interrupt!(com1, {
    use arch::driver::serial::COM1;
    info!("\nInterupt: COM1");
    COM1.lock().receive();
    ack(IRQ_COM1);
});

interrupt!(com2, {
    use arch::driver::serial::COM2;
    info!("\nInterupt: COM2");
    COM2.lock().receive();
    ack(IRQ_COM2);
});

interrupt_switch!(timer, stack, rsp, {
    use process;
    process::timer_handler(stack, &mut rsp);
    ack(IRQ_TIMER);
});

interrupt_stack_p!(to_user, stack, {
    use arch::gdt;
    info!("\nInterupt: To User");
    let rsp = unsafe{ (stack as *const InterruptStackP).offset(1) } as usize;
    gdt::set_ring0_rsp(rsp);
    stack.iret.cs = gdt::UCODE_SELECTOR.0 as usize;
    stack.iret.ss = gdt::UDATA_SELECTOR.0 as usize;
    stack.iret.rflags |= 3 << 12;   // 设置EFLAG的I/O特权位，使得在用户态可使用in/out指令
});

interrupt_stack_p!(to_kernel, stack, {
//    info!("rsp @ {:#x}", stack as *const _ as usize);
    use arch::gdt;
    info!("\nInterupt: To Kernel");
    stack.iret.cs = gdt::KCODE_SELECTOR.0 as usize;
    stack.iret.ss = gdt::KDATA_SELECTOR.0 as usize;
});

interrupt_switch!(syscall, stack, rsp, {
    info!("\nInterupt: Syscall {:#x?}", stack.scratch.rax);
    use syscall::syscall;
    let ret = syscall(stack, &mut rsp, false);
    stack.scratch.rax = ret as usize;
});

interrupt_switch!(syscall32, stack, rsp, {
//    info!("\nInterupt: Syscall {:#x?}", stack.scratch.rax);
    use syscall::syscall;
    let ret = syscall(stack, &mut rsp, true);
    stack.scratch.rax = ret as usize;
});