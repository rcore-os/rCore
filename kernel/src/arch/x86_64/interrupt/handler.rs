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
use crate::drivers::{DRIVERS, IRQ_MANAGER};
use bitflags::*;
use log::*;

global_asm!(include_str!("trap.asm"));
global_asm!(include_str!("vector.asm"));

#[allow(non_upper_case_globals)]
#[no_mangle]
pub extern "C" fn rust_trap(tf: &mut TrapFrame) {
    trace!(
        "Interrupt: {:#x} @ CPU{}",
        tf.trap_num,
        super::super::cpu::id()
    );
    // Dispatch
    match tf.trap_num as u8 {
        Breakpoint => breakpoint(),
        DoubleFault => double_fault(tf),
        PageFault => page_fault(tf),
        IRQ0..=63 => {
            let irq = tf.trap_num as u8 - IRQ0;
            super::ack(irq); // must ack before switching
            match irq {
                Timer => crate::trap::timer(),
                Keyboard => keyboard(),
                COM1 => com1(),
                COM2 => com2(),
                IDE => ide(),
                _ => {
                    if IRQ_MANAGER.read().try_handle_interrupt(Some(irq.into())) {
                        debug!("driver processed interrupt");
                        return;
                    }
                    warn!("unhandled external IRQ number: {}", irq);
                }
            }
        }
        Syscall32 => syscall32(tf),
        InvalidOpcode => invalid_opcode(tf),
        DivideError | GeneralProtectionFault => error(tf),
        IPIFuncCall => {
            let irq = tf.trap_num as u8 - IRQ0;
            super::ack(irq); // must ack before switching
            super::super::gdt::Cpu::current().handle_ipi();
        }
        _ => panic!("Unhandled interrupt {:x}", tf.trap_num),
    }
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
    unsafe {
        asm!("mov %cr2, $0" : "=r" (addr));
    }

    bitflags! {
        struct PageError: u8 {
            const PRESENT = 1 << 0;
            const WRITE = 1 << 1;
            const USER = 1 << 2;
            const RESERVED_WRITE = 1 << 3;
            const INST = 1 << 4;
        }
    }
    let code = PageError::from_bits(tf.error_code as u8).unwrap();

    if crate::memory::handle_page_fault(addr) {
        return;
    }

    extern "C" {
        fn _copy_user_start();
        fn _copy_user_end();
    }
    if tf.rip >= _copy_user_start as usize && tf.rip < _copy_user_end as usize {
        debug!("fixup for addr {:x?}", addr);
        tf.rip = crate::memory::read_user_fixup as usize;
        return;
    }

    error!("\nEXCEPTION: Page Fault @ {:#x}, code: {:?}", addr, code);
    error(tf);
}

fn keyboard() {
    use crate::arch::driver::keyboard;
    use pc_keyboard::{DecodedKey, KeyCode};
    trace!("\nInterupt: Keyboard");
    if let Some(key) = keyboard::receive() {
        match key {
            DecodedKey::Unicode(c) => crate::trap::serial(c),
            DecodedKey::RawKey(code) => {
                let s = match code {
                    KeyCode::ArrowUp => "\u{1b}[A",
                    KeyCode::ArrowDown => "\u{1b}[B",
                    KeyCode::ArrowRight => "\u{1b}[C",
                    KeyCode::ArrowLeft => "\u{1b}[D",
                    _ => "",
                };
                for c in s.chars() {
                    crate::trap::serial(c);
                }
            }
        }
    }
}

fn com1() {
    use crate::arch::driver::serial::*;
    trace!("\nInterupt: COM1");
    crate::trap::serial(COM1.lock().receive() as char);
}

fn com2() {
    use crate::arch::driver::serial::*;
    trace!("\nInterupt: COM2");
    COM2.lock().receive();
}

fn ide() {
    trace!("\nInterupt: IDE");
}

#[no_mangle]
pub extern "C" fn syscall(tf: &mut TrapFrame) {
    trace!("\nInterupt: Syscall {:#x?}", tf.rax);
    let ret = crate::syscall::syscall(tf.rax, [tf.rdi, tf.rsi, tf.rdx, tf.r10, tf.r8, tf.r9], tf);
    tf.rax = ret as usize;
}

fn syscall32(tf: &mut TrapFrame) {
    trace!("\nInterupt: Syscall {:#x?}", tf.rax);
    let ret = crate::syscall::syscall(tf.rax, [tf.rdx, tf.rcx, tf.rbx, tf.rdi, tf.rsi, 0], tf);
    tf.rax = ret as usize;
}

/// Support `syscall` instruction
fn invalid_opcode(tf: &mut TrapFrame) {
    let opcode = unsafe { (tf.rip as *mut u16).read() };
    const SYSCALL_OPCODE: u16 = 0x05_0f;
    if opcode == SYSCALL_OPCODE {
        tf.rip += 2; // must before syscall
        syscall(tf);
    } else {
        crate::trap::error(tf);
    }
}

fn error(tf: &TrapFrame) {
    crate::trap::error(tf);
}

#[no_mangle]
pub unsafe extern "C" fn set_return_rsp(tf: *const TrapFrame) {
    use crate::arch::gdt::Cpu;
    Cpu::current().set_ring0_rsp(tf.add(1) as usize);
}
