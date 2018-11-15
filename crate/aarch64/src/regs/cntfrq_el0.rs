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

//! Counter-timer Frequency register - EL0
//!
//! This register is provided so that software can discover the frequency of the
//! system counter. It must be programmed with this value as part of system
//! initialization. The value of the register is not interpreted by hardware.

use register::cpu::RegisterReadOnly;

pub struct Reg;

impl RegisterReadOnly<u32, ()> for Reg {
    sys_coproc_read_raw!(u32, "CNTFRQ_EL0");
}

pub static CNTFRQ_EL0: Reg = Reg {};
