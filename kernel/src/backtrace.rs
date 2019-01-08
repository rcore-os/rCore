extern "C" {
    fn stext();
    fn etext();
}

/// Returns the current frame pointer.
#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub fn fp() -> usize {
    let ptr: usize;
    unsafe {
        asm!("mov $0, x29" : "=r"(ptr));
    }

    ptr
}

/// Returns the current link register.
#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub fn lr() -> usize {
    let ptr: usize;
    unsafe {
        asm!("mov $0, x30" : "=r"(ptr));
    }

    ptr
}

// Print the backtrace starting from the caller
pub fn backtrace() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut current_pc = lr();
        let mut current_fp = fp();
        let mut stack_num = 0;
        println!("pc {:#018X} fp {:#018X}", current_pc, current_fp);
        while current_pc >= stext as usize && current_pc <= etext as usize && current_fp as usize != 0 {
            println!("#{} {:#018X}", stack_num, current_pc);
            stack_num = stack_num + 1;
            current_fp = *(current_fp as *const usize);
            if current_fp as usize != 0 {
                current_pc = *(current_fp as *const usize).offset(1);
            }
            println!("pc {:#018X} fp {:#018X}", current_pc, current_fp);
        }
    }
}
