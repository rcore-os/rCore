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

//! Counter-timer Physical Timer TimerValue register - EL0
//!
//! Holds the timer value for the EL1 physical timer.

use register::cpu::RegisterReadWrite;

pub struct Reg;

impl RegisterReadWrite<u32, ()> for Reg {
    sys_coproc_read_raw!(u32, "CNTP_TVAL_EL0");
    sys_coproc_write_raw!(u32, "CNTP_TVAL_EL0");
}

pub static CNTP_TVAL_EL0: Reg = Reg {};
