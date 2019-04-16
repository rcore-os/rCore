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
    #[cfg(any(target_arch = "mips"))]
    unsafe {
        // read $sp
        asm!("ori $0, $$29, 0" : "=r"(ptr));
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

    #[cfg(target_arch = "mips")]
    unsafe {
        asm!("ori $0, $$31, 0" : "=r"(ptr));
    }

    ptr
}

// Print the backtrace starting from the caller
pub fn backtrace() {
    unsafe {
        let mut current_pc = lr();
        let mut current_fp = fp();
        let mut stack_num = 0;

        // adjust sp to the top address of backtrace() function
        #[cfg(target_arch = "mips")]
        {
            let func_base = backtrace as *const isize;
            let sp_offset = (*func_base << 16) >> 16;
            current_fp = ((current_fp as isize) - sp_offset) as usize;
        }

        println!("=== BEGIN rCore stack trace ===");

        while current_pc >= stext as usize
            && current_pc <= etext as usize
            && current_fp as usize != 0
        {
            // print current backtrace
            match size_of::<usize>() {
                4 => {
                    println!(
                        "#{:02} PC: {:#010X} FP: {:#010X}",
                        stack_num,
                        current_pc - size_of::<usize>(),
                        current_fp
                    );
                }
                _ => {
                    println!(
                        "#{:02} PC: {:#018X} FP: {:#018X}",
                        stack_num,
                        current_pc - size_of::<usize>(),
                        current_fp
                    );
                }
            }

            stack_num = stack_num + 1;
            #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))]
            {
                current_fp = *(current_fp as *const usize).offset(-2);
                current_pc = *(current_fp as *const usize).offset(-1);
            }
            #[cfg(target_arch = "aarch64")]
            {
                current_fp = *(current_fp as *const usize);
                if current_fp < crate::arch::consts::KERNEL_OFFSET {
                    break;
                }
                if current_fp != 0 {
                    current_pc = *(current_fp as *const usize).offset(1);
                }
            }
            #[cfg(target_arch = "mips")]
            {
                // the prologue of function is always like:
                // main+0: 27bd____ addiu sp, sp, -____
                // main+4: afbf____ sw    ra, ____(sp)
                let mut code_ptr = current_pc as *const isize;
                code_ptr = code_ptr.offset(-1);

                // get the stack size of last function
                while (*code_ptr as usize >> 16) != 0x27bd {
                    code_ptr = code_ptr.offset(-1);
                }
                let sp_offset = (*code_ptr << 16) >> 16;
                trace!(
                    "Found addiu sp @ {:08X}({:08x}) with sp offset {}",
                    code_ptr as usize,
                    *code_ptr,
                    sp_offset
                );

                // get the return address offset of last function
                let mut last_fun_found = false;
                while (code_ptr as usize) < current_pc {
                    if (*code_ptr as usize >> 16) == 0xafbf {
                        last_fun_found = true;
                        break;
                    }
                    code_ptr = code_ptr.offset(1);
                }
                if last_fun_found {
                    // unwind stack
                    let ra_offset = (*code_ptr << 16) >> 16;
                    trace!(
                        "Found sw ra @ {:08X}({:08x}) with ra offset {}",
                        code_ptr as usize,
                        *code_ptr,
                        ra_offset
                    );
                    current_pc = *(((current_fp as isize) + ra_offset) as *const usize);
                    current_fp = ((current_fp as isize) - sp_offset) as usize;
                    trace!("New PC {:08X} FP {:08X}", current_pc, current_fp);
                    continue;
                } else {
                    trace!("No sw ra found, probably due to optimizations.");
                    break;
                }
            }
            #[cfg(target_arch = "x86_64")]
            {
                // Kernel stack at 0x0000_57ac_0000_0000 (defined in bootloader crate)
                // size = 512 pages
                current_fp = *(current_fp as *const usize).offset(0);
                if current_fp >= 0x0000_57ac_0000_0000 + 512 * PAGE_SIZE - size_of::<usize>()
                    && current_fp <= 0xffff_ff00_0000_0000
                {
                    break;
                }
                current_pc = *(current_fp as *const usize).offset(1);
            }
        }
        println!("=== END rCore stack trace ===");
    }
}
