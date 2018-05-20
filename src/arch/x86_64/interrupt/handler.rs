fn breakpoint() {
    error!("\nEXCEPTION: Breakpoint");
}

fn double_fault() {
    error!("\nEXCEPTION: Double Fault");
    loop {}
}

fn page_fault(tf: &mut TrapFrame) {
    use x86_64::registers::control_regs::cr2;
    let addr = cr2().0;
    error!("\nEXCEPTION: Page Fault @ {:#x}, code: {:#x}", addr, tf.error_code);

    use memory::page_fault_handler;
    if page_fault_handler(addr) {
        return;
    }

    loop {}
}

fn general_protection_fault() {
    error!("\nEXCEPTION: General Protection Fault");
    loop {}
}

fn invalid_opcode() {
    error!("\nEXCEPTION: Invalid Opcode");
    loop {}
}

#[cfg(feature = "use_apic")]
use arch::driver::apic::ack;
#[cfg(not(feature = "use_apic"))]
use arch::driver::pic::ack;

use super::consts::*;

fn keyboard() {
    use arch::driver::keyboard;
    info!("\nInterupt: Keyboard");
    let c = keyboard::get();
    info!("Key = '{}' {}", c as u8 as char, c);
}

fn com1() {
    use arch::driver::serial::COM1;
    info!("\nInterupt: COM1");
    COM1.lock().receive();
}

fn com2() {
    use arch::driver::serial::COM2;
    info!("\nInterupt: COM2");
    COM2.lock().receive();
}

fn timer(tf: &mut TrapFrame, rsp: &mut usize) {
    use process;
    process::timer_handler(tf, rsp);
}

fn to_user(tf: &mut TrapFrame) {
    use arch::gdt;
    info!("\nInterupt: To User");
    tf.cs = gdt::UCODE_SELECTOR.0 as usize;
    tf.ss = gdt::UDATA_SELECTOR.0 as usize;
    tf.rflags |= 3 << 12;   // 设置EFLAG的I/O特权位，使得在用户态可使用in/out指令
}

fn to_kernel(tf: &mut TrapFrame) {
    use arch::gdt;
    info!("\nInterupt: To Kernel");
    tf.cs = gdt::KCODE_SELECTOR.0 as usize;
    tf.ss = gdt::KDATA_SELECTOR.0 as usize;
}

fn syscall(tf: &mut TrapFrame, rsp: &mut usize) {
    info!("\nInterupt: Syscall {:#x?}", tf.rax);
    use syscall::syscall;
    let ret = syscall(tf, rsp, false);
    tf.rax = ret as usize;
}

fn syscall32(tf: &mut TrapFrame, rsp: &mut usize) {
    //    info!("\nInterupt: Syscall {:#x?}", tf.rax);
    use syscall::syscall;
    let ret = syscall(tf, rsp, true);
    tf.rax = ret as usize;
}

#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) -> usize {
    let mut rsp = tf as *const _ as usize;

    // Dispatch
    match tf.trap_num as u8 {
        T_BRKPT => breakpoint(),
        T_DBLFLT => double_fault(),
        T_PGFLT => page_fault(tf),
        T_GPFLT => general_protection_fault(),
        T_IRQ0...64 => {
            let irq = tf.trap_num as u8 - T_IRQ0;
            match irq {
                IRQ_TIMER => timer(tf, &mut rsp),
                IRQ_KBD => keyboard(),
                IRQ_COM1 => com1(),
                IRQ_COM2 => com2(),
                _ => panic!("Invalid IRQ number."),
            }
            ack(irq);
        }
        T_SWITCH_TOK => to_kernel(tf),
        T_SWITCH_TOU => to_user(tf),
        T_SYSCALL => syscall(tf, &mut rsp),
        0x80 => syscall32(tf, &mut rsp),
        _ => panic!("Unhandled interrupt {:x}", tf.trap_num),
    }

    // Set return rsp if to user
    let tf = unsafe { &*(rsp as *const TrapFrame) };
    set_return_rsp(tf);

    rsp
}

fn set_return_rsp(tf: &TrapFrame) {
    use arch::gdt::Cpu;
    use core::mem::size_of;
    if tf.cs & 0x3 == 3 {
        Cpu::current().set_ring0_rsp(tf as *const _ as usize + size_of::<TrapFrame>());
    }
}

#[derive(Debug, Clone, Default)]
pub struct TrapFrame {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,

    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,

    pub trap_num: usize,
    pub error_code: usize,

    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,

    pub rsp: usize,
    pub ss: usize,
}