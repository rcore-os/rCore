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

//! Counter-timer Hypervisor Control register - EL2
//!
//! Controls the generation of an event stream from the physical counter, and
//! access from Non-secure EL1 to the physical counter and the Non-secure EL1
//! physical timer.

use register::cpu::RegisterReadWrite;

// When HCR_EL2.E2H == 0:
// TODO: Figure out how we can differentiate depending on HCR_EL2.E2H state
//
// For now, implement the HCR_EL2.E2H == 0 version
register_bitfields! {u32,
    CNTHCTL_EL2 [
        /// Traps Non-secure EL0 and EL1 accesses to the physical timer
        /// registers to EL2.
        ///
        /// 0 From AArch64 state: Non-secure EL0 and EL1 accesses to the
        ///   CNTP_CTL_EL0, CNTP_CVAL_EL0, and CNTP_TVAL_EL0 are trapped to EL2,
        ///   unless it is trapped by CNTKCTL_EL1.EL0PTEN.
        ///
        ///   From AArch32 state: Non-secure EL0 and EL1 accesses to the
        ///   CNTP_CTL, CNTP_CVAL, and CNTP_TVAL are trapped to EL2, unless it
        ///   is trapped by CNTKCTL_EL1.EL0PTEN or CNTKCTL.PL0PTEN.
        ///
        /// 1 This control does not cause any instructions to be trapped.
        ///
        /// If EL3 is implemented and EL2 is not implemented, behavior is as if
        /// this bit is 1 other than for the purpose of a direct read.
        EL1PCEN  OFFSET(1) NUMBITS(1) [],

        /// Traps Non-secure EL0 and EL1 accesses to the physical counter
        /// register to EL2.
        ///
        /// 0 From AArch64 state: Non-secure EL0 and EL1 accesses to the
        ///   CNTPCT_EL0 are trapped to EL2, unless it is trapped by
        ///   CNTKCTL_EL1.EL0PCTEN.
        ///
        ///   From AArch32 state: Non-secure EL0 and EL1 accesses to the CNTPCT
        ///   are trapped to EL2, unless it is trapped by CNTKCTL_EL1.EL0PCTEN
        ///   or CNTKCTL.PL0PCTEN.
        ///
        /// 1 This control does not cause any instructions to be trapped.
        ///
        /// If EL3 is implemented and EL2 is not implemented, behavior is as if
        /// this bit is 1 other than for the purpose of a direct read.
        EL1PCTEN OFFSET(0) NUMBITS(1) []
    ]
}

pub struct Reg;

impl RegisterReadWrite<u32, CNTHCTL_EL2::Register> for Reg {
    sys_coproc_read_raw!(u32, "CNTHCTL_EL2");
    sys_coproc_write_raw!(u32, "CNTHCTL_EL2");
}

#[allow(non_upper_case_globals)]
pub static CNTHCTL_EL2: Reg = Reg {};
