use core::mem::size_of;

extern "C" {
    fn stext();
    fn etext();
}

/// Returns the current frame pointer.
#[inline(always)]
#[cfg(any(target_arch = "aarch64", target_arch = "riscv32", target_arch = "riscv64"))]
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

    ptr
}

/// Returns the current link register.
#[inline(always)]
#[cfg(any(target_arch = "aarch64", target_arch = "riscv32", target_arch = "riscv64"))]
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

    ptr
}

// Print the backtrace starting from the caller
pub fn backtrace() {
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv32", target_arch = "riscv64"))]
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
        }
    }
}
