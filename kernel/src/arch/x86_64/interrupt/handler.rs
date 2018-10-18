//! 中断服务例程
//!
//! # 中断处理总流程
//!
//! * 中断发生时，CPU首先完成以下工作：
//!     * 设置rsp的值（Ring3->0 或 DoubleFault，在TSS中设置）
//!     * 向栈中push一些值（Ring0时3项，Ring3时5项，某些中断还有错误码）
//!     * 跳转到`vector{i}`符号（位于`vector.asm`）
//! * 进入中断服务例程后，完成以下工作：
//!     * `vector{i}`向栈中push空错误码（如CPU没有做）和中断号
//!     * 跳转到`__alltraps`（位于`trap.asm`）
//!     * `__alltraps`向栈中push全部寄存器的值
//!     * 调用`rust_trap`，并把此时的rsp传过来，也就是`TrapFrame`的指针
//! * `rust_trap`完成以下工作：
//!     * 根据tf中的中断号，再次分发到具体的中断处理函数。
//!       一些函数可能会调用switch()，切换到其它线程执行，在某一时刻再切换回来。
//!     * 如果需要，执行进程调度
//! * `__alltraps`完成以下工作：
//!     * 检查tf中的cs段，如果是用户态，向TSS中设置下一次中断时重置rsp的值，使其指向该用户线程的内核栈
//!     * 从栈中pop全部寄存器的值，pop中断号和错误码
//!     * 执行iret，CPU从栈中pop一些值（若其中的CS是Ring0，3项，否则5项），重置rip和rsp（Ring3时）
//!
//! ``` mermaid
//! sequenceDiagram
//!	activate CPU
//!	Note right of CPU: set rsp (user)
//!	Note right of CPU: push something
//!	deactivate CPU
//!
//!	CPU ->> +ASM: jmp vector_i
//!	Note right of ASM: push error_code & trap_num
//!
//!	ASM ->> ASM: jmp alltraps
//!	Note right of ASM: push registers
//!
//!	ASM ->> +Rust: call rust_trap(rsp)
//!    Note right of Rust: handle the trap
//!
//!	opt schedule
//!	Rust ->> +NewThread: switch
//!	Note left of NewThread: forkret:
//!    NewThread -->> -ASM: ret
//!    Note left of ASM: trapret:
//!    ASM -->> +NewThread: from another trap
//!    NewThread ->> -Rust: switch
//!	end
//!
//!	Rust -->> -ASM: ret
//!    Note left of ASM: trapret:
//!
//!    opt CS is user mode
//!	ASM ->> +Rust: call set_return_rsp
//!	Note right of Rust: set in TSS
//!	Rust -->> -ASM: ret
//!	end
//!
//!	Note right of ASM: pop registers
//!	Note right of ASM: pop error_code & trap_num
//!	ASM -->> -CPU: iret
//!
//!	activate CPU
//!	Note right of CPU: pop something
//!	Note right of CPU: set rsp (user)
//!	deactivate CPU
//! ```

use super::consts::*;
use super::TrapFrame;

global_asm!(include_str!("trap.asm"));
global_asm!(include_str!("vector.asm"));

#[no_mangle]
pub extern fn rust_trap(tf: &mut TrapFrame) {
    trace!("Interrupt: {:#x} @ CPU{}", tf.trap_num, super::super::cpu::id());
    // Dispatch
    match tf.trap_num as u8 {
        T_BRKPT => breakpoint(),
        T_DBLFLT => double_fault(tf),
        T_PGFLT => page_fault(tf),
        T_IRQ0...63 => {
            let irq = tf.trap_num as u8 - T_IRQ0;
            match irq {
                IRQ_TIMER => ::trap::timer(),
                IRQ_KBD => keyboard(),
                IRQ_COM1 => com1(),
                IRQ_COM2 => com2(),
                IRQ_IDE => ide(),
                _ => panic!("Invalid IRQ number: {}", irq),
            }
            super::ack(irq);
        }
        T_SWITCH_TOK => to_kernel(tf),
        T_SWITCH_TOU => to_user(tf),
        T_SYSCALL => syscall(tf),
        T_SYSCALL32 => syscall32(tf),
        T_DIVIDE | T_GPFLT | T_ILLOP => error(tf),
        _ => panic!("Unhandled interrupt {:x}", tf.trap_num),
    }
    ::trap::before_return();
}

fn breakpoint() {
    error!("\nEXCEPTION: Breakpoint");
}

fn double_fault(tf: &TrapFrame) {
    error!("\nEXCEPTION: Double Fault\n{:#x?}", tf);
    loop {}
}

fn page_fault(tf: &mut TrapFrame) {
    let addr: usize;
    unsafe { asm!("mov %cr2, $0" : "=r" (addr)); }
    error!("\nEXCEPTION: Page Fault @ {:#x}, code: {:#x}", addr, tf.error_code);

    use memory::page_fault_handler;
    if page_fault_handler(addr) {
        return;
    }

    error(tf);
}

fn keyboard() {
    use arch::driver::keyboard;
    info!("\nInterupt: Keyboard");
    let c = keyboard::get();
    info!("Key = '{}' {}", c as u8 as char, c);
}

fn com1() {
    use arch::driver::serial::*;
    trace!("\nInterupt: COM1");
}

fn com2() {
    use arch::driver::serial::*;
    trace!("\nInterupt: COM2");
    COM2.lock().receive();
}

fn ide() {
    trace!("\nInterupt: IDE");
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

fn syscall(tf: &mut TrapFrame) {
    trace!("\nInterupt: Syscall {:#x?}", tf.rax);
    use syscall::syscall;
    let ret = syscall(tf.rax, [tf.rdi, tf.rsi, tf.rdx, tf.rcx, tf.r8, tf.r9], tf);
    tf.rax = ret as usize;
}

fn syscall32(tf: &mut TrapFrame) {
    trace!("\nInterupt: Syscall {:#x?}", tf.rax);
    use syscall::syscall;
    let ret = syscall(tf.rax, [tf.rdx, tf.rcx, tf.rbx, tf.rdi, tf.rsi, 0], tf);
    tf.rax = ret as usize;
}

fn error(tf: &TrapFrame) {
    ::trap::error(tf);
}

#[no_mangle]
pub extern fn set_return_rsp(tf: &TrapFrame) {
    use arch::gdt::Cpu;
    use core::mem::size_of;
    if tf.cs & 0x3 == 3 {
        Cpu::current().set_ring0_rsp(tf as *const _ as usize + size_of::<TrapFrame>());
    }
}
