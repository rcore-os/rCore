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

//! The stack pointer - EL1
//!
//! Holds the stack pointer associated with EL1. When executing at EL1, the
//! value of SPSel.SP determines the current stack pointer:
//!
//! SPSel.SP | current stack pointer
//! --------------------------------
//! 0        | SP_EL0
//! 1        | SP_EL1

use register::cpu::RegisterReadWrite;

pub struct Reg;

impl RegisterReadWrite<u64, ()> for Reg {
    sys_coproc_read_raw!(u64, "SP_EL1");
    sys_coproc_write_raw!(u64, "SP_EL1");
}

pub static SP_EL1: Reg = Reg {};
