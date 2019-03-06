use core::mem::size_of;
use rcore_memory::PAGE_SIZE;

extern "C" {
    fn stext();
    fn etext();
}

/// Returns the current frame pointer.or stack base pointer
#[inline(always)]
pub fn fp() -> usize {
    let ptr: usize;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("mov $0, x29" : "=r"(ptr));
    }
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        asm!("mv $0, s0" : "=r"(ptr));
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("mov %rbp, $0" : "=r"(ptr));
    }

    ptr
}

/// Returns the current link register.or return address
#[inline(always)]
pub fn lr() -> usize {
    let ptr: usize;
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("mov $0, x30" : "=r"(ptr));
    }
    #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
    unsafe {
        asm!("mv $0, ra" : "=r"(ptr));
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        asm!("movq 8(%rbp), $0" : "=r"(ptr));
    }

    ptr
}

// Print the backtrace starting from the caller
pub fn backtrace() {
    unsafe {
        let mut current_pc = lr();
        let mut current_fp = fp();
        let mut stack_num = 0;
        while current_pc >= stext as usize && current_pc <= etext as usize && current_fp as usize != 0 {
            println!("#{} {:#018X} fp {:#018X}", stack_num, current_pc - size_of::<usize>(), current_fp);
            stack_num = stack_num + 1;
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            {
                current_fp = *(current_fp as *const usize).offset(-2);
                current_pc = *(current_fp as *const usize).offset(-1);
            }
            #[cfg(target_arch = "aarch64")]
            {
                current_fp = *(current_fp as *const usize);
                if current_fp != 0 {
                    current_pc = *(current_fp as *const usize).offset(1);
                }
            }
            #[cfg(target_arch = "x86_64")]
            {
                // Kernel stack at 0x0000_57ac_0000_0000 (defined in bootloader crate)
                // size = 512 pages
                current_fp = *(current_fp as *const usize).offset(0);
                if current_fp >= 0x0000_57ac_0000_0000 + 512 * PAGE_SIZE - size_of::<usize>() &&
                    current_fp <= 0xffff_ff00_0000_0000 {
                    break;
                }
                current_pc = *(current_fp as *const usize).offset(1);
            }
        }
    }
}
