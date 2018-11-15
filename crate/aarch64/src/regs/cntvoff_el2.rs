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

//! Counter-timer Virtual Offset register - EL2
//!
//! Holds the 64-bit virtual offset. This is the offset between the physical
//! count value visible in CNTPCT_EL0 and the virtual count value visible in
//! CNTVCT_EL0.

use register::cpu::RegisterReadWrite;

pub struct Reg;

impl RegisterReadWrite<u64, ()> for Reg {
    sys_coproc_read_raw!(u64, "CNTVOFF_EL2");
    sys_coproc_write_raw!(u64, "CNTVOFF_EL2");
}

pub static CNTVOFF_EL2: Reg = Reg {};
