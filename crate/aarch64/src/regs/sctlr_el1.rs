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

//! System Control Register - EL1
//!
//! Provides top level control of the system, including its memory system, at
//! EL1 and EL0.

use register::cpu::RegisterReadWrite;

register_bitfields! {u32,
    SCTLR_EL1 [
        /// Instruction access Cacheability control, for accesses at EL0 and
        /// EL1:
        ///
        /// 0 All instruction access to Normal memory from EL0 and EL1 are
        ///   Non-cacheable for all levels of instruction and unified cache.
        ///
        ///   If the value of SCTLR_EL1.M is 0, instruction accesses from stage
        ///   1 of the EL1&0 translation regime are to Normal, Outer Shareable,
        ///   Inner Non-cacheable, Outer Non-cacheable memory.
        ///
        /// 1 This control has no effect on the Cacheability of instruction
        ///   access to Normal memory from EL0 and EL1.
        ///
        ///   If the value of SCTLR_EL1.M is 0, instruction accesses from stage
        ///   1 of the EL1&0 translation regime are to Normal, Outer Shareable,
        ///   Inner Write-Through, Outer Write-Through memory.
        ///
        /// When the value of the HCR_EL2.DC bit is 1, then instruction access
        /// to Normal memory from EL0 and EL1 are Cacheable regardless of the
        /// value of the SCTLR_EL1.I bit.
        ///
        /// When ARMv8.1-VHE is implemented, and the value of HCR_EL2.{E2H, TGE}
        /// is {1, 1}, this bit has no effect on the PE.
        ///
        /// When this register has an architecturally-defined reset value, this
        /// field resets to 0.
        I OFFSET(12) NUMBITS(1) [
            NonCacheable = 0,
            Cacheable = 1
        ],

        /// Cacheability control, for data accesses.
        ///
        /// 0 All data access to Normal memory from EL0 and EL1, and all Normal
        ///   memory accesses to the EL1&0 stage 1 translation tables, are
        ///   Non-cacheable for all levels of data and unified cache.
        ///
        /// 1 This control has no effect on the Cacheability of:
        ///     - Data access to Normal memory from EL0 and EL1.
        ///     - Normal memory accesses to the EL1&0 stage 1 translation
        ///       tables.
        ///
        /// When the value of the HCR_EL2.DC bit is 1, the PE ignores
        /// SCLTR.C. This means that Non-secure EL0 and Non-secure EL1 data
        /// accesses to Normal memory are Cacheable.
        ///
        /// When ARMv8.1-VHE is implemented, and the value of HCR_EL2.{E2H, TGE}
        /// is {1, 1}, this bit has no effect on the PE.
        ///
        /// When this register has an architecturally-defined reset value, this
        /// field resets to 0.
        C OFFSET(2) NUMBITS(1) [
            NonCacheable = 0,
            Cacheable = 1
        ],

        /// MMU enable for EL1 and EL0 stage 1 address translation. Possible
        /// values of this bit are:
        ///
        /// 0 EL1 and EL0 stage 1 address translation disabled.
        ///   See the SCTLR_EL1.I field for the behavior of instruction accesses
        ///   to Normal memory.
        /// 1 EL1 and EL0 stage 1 address translation enabled.
        M OFFSET(0) NUMBITS(1) [
            Disable = 0,
            Enable = 1
        ]
    ]
}

pub struct Reg;

impl RegisterReadWrite<u32, SCTLR_EL1::Register> for Reg {
    sys_coproc_read_raw!(u32, "SCTLR_EL1");
    sys_coproc_write_raw!(u32, "SCTLR_EL1");
}

pub static SCTLR_EL1: Reg = Reg {};
