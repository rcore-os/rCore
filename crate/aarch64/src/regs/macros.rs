/*
 * Copyright (c) 2018 by the author(s)
 *
 * =============================================================================
 *
 * Licensed under either of
 *   - Apache License, Version 2.0 (http://www.apache.org/licenses/LICENSE-2.0)
 *   - MIT License (http://opensource.org/licenses/MIT)
 * at your option.
 *
 * =============================================================================
 *
 * Author(s):
 *   - Andre Richter <andre.o.richter@gmail.com>
 */

macro_rules! __read_raw {
    ($width:ty, $asm_instr:tt, $asm_reg_name:tt) => {
        /// Reads the raw bits of the CPU register.
        #[inline]
        fn get(&self) -> $width {
            match () {
                #[cfg(target_arch = "aarch64")]
                () => {
                    let reg;
                    unsafe {
                        asm!(concat!($asm_instr, " $0, ", $asm_reg_name) : "=r"(reg) ::: "volatile");
                    }
                    reg
                }

                #[cfg(not(target_arch = "aarch64"))]
                () => unimplemented!(),
            }
        }
    };
}

macro_rules! __write_raw {
    ($width:ty, $asm_instr:tt, $asm_reg_name:tt) => {
        /// Writes raw bits to the CPU register.
        #[cfg_attr(not(target_arch = "aarch64"), allow(unused_variables))]
        #[inline]
        fn set(&self, value: $width) {
            match () {
                #[cfg(target_arch = "aarch64")]
                () => {
                    unsafe {
                        asm!(concat!($asm_instr, " ", $asm_reg_name, ", $0") :: "r"(value) :: "volatile")
                    }
                }

                #[cfg(not(target_arch = "aarch64"))]
                () => unimplemented!(),
            }
        }
    };
}

/// Raw read from system coprocessor registers
macro_rules! sys_coproc_read_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __read_raw!($width, "mrs", $asm_reg_name);
    };
}

/// Raw write to system coprocessor registers
macro_rules! sys_coproc_write_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __write_raw!($width, "msr", $asm_reg_name);
    };
}

/// Raw read from (ordinary) registers
macro_rules! read_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __read_raw!($width, "mov", $asm_reg_name);
    };
}
/// Raw write to (ordinary) registers
macro_rules! write_raw {
    ($width:ty, $asm_reg_name:tt) => {
        __write_raw!($width, "mov", $asm_reg_name);
    };
}
