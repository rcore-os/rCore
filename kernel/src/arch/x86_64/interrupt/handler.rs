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
use crate::drivers::IRQ_MANAGER;
use bitflags::*;
use log::*;
use x86_64::registers::control::Cr2;

#[allow(non_upper_case_globals)]
#[no_mangle]
pub extern "C" fn trap_handler(tf: &mut TrapFrame) {
    trace!(
        "Interrupt: {:#x} @ CPU{}",
        tf.trap_num,
        super::super::cpu::id()
    );

    // Dispatch
    match tf.trap_num {
        DoubleFault => double_fault(tf),
        PageFault => page_fault(tf),
        IrqMin..=IrqMax => {
            let irq = tf.trap_num - IrqMin;
            super::ack(irq); // must ack before switching
            match tf.trap_num {
                Timer => {
                    crate::trap::timer();
                }
                _ => {
                    if IRQ_MANAGER.read().try_handle_interrupt(Some(irq)) {
                        trace!("driver processed interrupt");
                        return;
                    }
                    warn!("unhandled external IRQ number: {}", irq);
                }
            }
        }
        IPIFuncCall => {
            let irq = tf.trap_num - IrqMin;
            super::ack(irq); // must ack before switching
            super::super::gdt::Cpu::current().handle_ipi();
        }
        _ => panic!("Unhandled interrupt {:x}", tf.trap_num),
    }
}

fn double_fault(tf: &TrapFrame) {
    error!("\nEXCEPTION: Double Fault\n{:#x?}", tf);
    loop {}
}

fn page_fault(tf: &mut TrapFrame) {
    let addr = Cr2::read().as_u64() as usize;

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
    loop {}
}
