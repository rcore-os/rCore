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

//! Hypervisor Configuration Register - EL2
//!
//! Provides configuration controls for virtualization, including defining
//! whether various Non-secure operations are trapped to EL2.

use register::cpu::RegisterReadWrite;

register_bitfields! {u64,
    HCR_EL2 [
        /// Execution state control for lower Exception levels:
        ///
        /// 0 Lower levels are all AArch32.
        /// 1 The Execution state for EL1 is AArch64. The Execution state for
        ///   EL0 is determined by the current value of PSTATE.nRW when
        ///   executing at EL0.
        ///
        /// If all lower Exception levels cannot use AArch32 then this bit is
        /// RAO/WI.
        ///
        /// In an implementation that includes EL3, when SCR_EL3.NS==0, the PE
        /// behaves as if this bit has the same value as the SCR_EL3.RW bit for
        /// all purposes other than a direct read or write access of HCR_EL2.
        ///
        /// The RW bit is permitted to be cached in a TLB.
        ///
        /// When ARMv8.1-VHE is implemented, and the value of HCR_EL2.{E2H, TGE}
        /// is {1, 1}, this field behaves as 1 for all purposes other than a
        /// direct read of the value of this bit.
        RW   OFFSET(31) NUMBITS(1) [
            AllLowerELsAreAarch32 = 0,
            EL1IsAarch64 = 1
        ],

        /// Default Cacheability.
        ///
        /// 0 This control has no effect on the Non-secure EL1&0 translation
        ///   regime.
        ///
        /// 1 In Non-secure state:
        ///     - When EL1 is using AArch64, the PE behaves as if the value of
        ///       the SCTLR_EL1.M field is 0 for all purposes other than
        ///       returning the value of a direct read of SCTLR_EL1.
        ///
        ///     - When EL1 is using AArch32, the PE behaves as if the value of
        ///       the SCTLR.M field is 0 for all purposes other than returning
        ///       the value of a direct read of SCTLR.
        ///
        ///     - The PE behaves as if the value of the HCR_EL2.VM field is 1
        ///       for all purposes other than returning the value of a direct
        ///       read of HCR_EL2.
        ///
        ///     - The memory type produced by stage 1 of the EL1&0 translation
        ///       regime is Normal Non-Shareable, Inner Write-Back Read-Allocate
        ///       Write-Allocate, Outer Write-Back Read-Allocate Write-Allocate.
        ///
        /// This field has no effect on the EL2, EL2&0, and EL3 translation
        /// regimes.
        ///
        /// This field is permitted to be cached in a TLB.
        ///
        /// In an implementation that includes EL3, when the value of SCR_EL3.NS
        /// is 0 the PE behaves as if this field is 0 for all purposes other
        /// than a direct read or write access of HCR_EL2.
        ///
        /// When ARMv8.1-VHE is implemented, and the value of HCR_EL2.{E2H, TGE}
        /// is {1, 1}, this field behaves as 0 for all purposes other than a
        /// direct read of the value of this field.
        DC   OFFSET(12) NUMBITS(1) [],

        /// Set/Way Invalidation Override. Causes Non-secure EL1 execution of
        /// the data cache invalidate by set/way instructions to perform a data
        /// cache clean and invalidate by set/way:
        ///
        /// 0 This control has no effect on the operation of data cache
        ///   invalidate by set/way instructions.
        ///
        /// 1 Data cache invalidate by set/way instructions perform a data cache
        ///   clean and invalidate by set/way.
        ///
        /// When the value of this bit is 1:
        ///
        /// AArch32: DCISW performs the same invalidation as a DCCISW
        ///          instruction.
        ///
        /// AArch64: DC ISW performs the same invalidation as a DC CISW
        ///          instruction.
        ///
        /// This bit can be implemented as RES 1.
        ///
        /// In an implementation that includes EL3, when the value of SCR_EL3.NS
        /// is 0 the PE behaves as if this field is 0 for all purposes other
        /// than a direct read or write access of HCR_EL2.
        ///
        /// When HCR_EL2.TGE is 1, the PE ignores the value of this field for
        /// all purposes other than a direct read of this field.
        SWIO OFFSET(1) NUMBITS(1) []
    ]
}

pub struct Reg;

impl RegisterReadWrite<u64, HCR_EL2::Register> for Reg {
    sys_coproc_read_raw!(u64, "HCR_EL2");
    sys_coproc_write_raw!(u64, "HCR_EL2");
}

pub static HCR_EL2: Reg = Reg {};
