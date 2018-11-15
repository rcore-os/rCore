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

//! Saved Program Status Register - EL2
//!
//! Holds the saved process state when an exception is taken to EL2.

use register::cpu::RegisterReadWrite;

register_bitfields! {u32,
    SPSR_EL2 [
        /// Process state D mask. The possible values of this bit are:
        ///
        /// 0 Watchpoint, Breakpoint, and Software Step exceptions targeted at
        ///   the current Exception level are not masked.
        ///
        /// 1 Watchpoint, Breakpoint, and Software Step exceptions targeted at
        ///   the current Exception level are masked.
        ///
        /// When the target Exception level of the debug exception is higher
        /// than the current Exception level, the exception is not masked by
        /// this bit.
        D OFFSET(9) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// SError interrupt mask bit. The possible values of this bit are:
        ///
        /// 0 Exception not masked.
        /// 1 Exception masked.
        A OFFSET(8) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// IRQ mask bit. The possible values of this bit are:
        ///
        /// 0 Exception not masked.
        /// 1 Exception masked.
        I OFFSET(7) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// FIQ mask bit. The possible values of this bit are:
        ///
        /// 0 Exception not masked.
        /// 1 Exception masked.
        F OFFSET(6) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// AArch64 state (Exception level and selected SP) that an exception
        /// was taken from. The possible values are:
        ///
        /// M[3:0] | State
        /// --------------
        /// 0b0000 | EL0t
        /// 0b0100 | EL1t
        /// 0b0101 | EL1h
        /// 0b1000 | EL2t
        /// 0b1001 | EL2h
        ///
        /// Other values are reserved, and returning to an Exception level that
        /// is using AArch64 with a reserved value in this field is treated as
        /// an illegal exception return.
        ///
        /// The bits in this field are interpreted as follows:
        ///   - M[3:2] holds the Exception Level.
        ///   - M[1] is unused and is RES 0 for all non-reserved values.
        ///   - M[0] is used to select the SP:
        ///     - 0 means the SP is always SP0.
        ///     - 1 means the exception SP is determined by the EL.
        M OFFSET(0) NUMBITS(4) [
            EL0t = 0b0000,
            EL1t = 0b0100,
            EL1h = 0b0101,
            EL2t = 0b1000,
            EL2h = 0b1001
        ]
    ]
}

pub struct Reg;

impl RegisterReadWrite<u32, SPSR_EL2::Register> for Reg {
    sys_coproc_read_raw!(u32, "SPSR_EL2");
    sys_coproc_write_raw!(u32, "SPSR_EL2");
}

pub static SPSR_EL2: Reg = Reg {};
