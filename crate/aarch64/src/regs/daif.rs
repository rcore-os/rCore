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

//! Interrupt Mask Bits
//!
//! Allows access to the interrupt mask bits.

use register::cpu::RegisterReadWrite;

register_bitfields! {u32,
    DAIF [
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
        ///
        /// When this register has an architecturally-defined reset value, this
        /// field resets to 1.
        D OFFSET(9) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// SError interrupt mask bit. The possible values of this bit are:
        ///
        /// 0 Exception not masked.
        /// 1 Exception masked.
        ///
        /// When this register has an architecturally-defined reset value, this
        /// field resets to 1 .
        A OFFSET(8) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// IRQ mask bit. The possible values of this bit are:
        ///
        /// 0 Exception not masked.
        /// 1 Exception masked.
        ///
        /// When this register has an architecturally-defined reset value, this
        /// field resets to 1 .
        I OFFSET(7) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ],

        /// FIQ mask bit. The possible values of this bit are:
        ///
        /// 0 Exception not masked.
        /// 1 Exception masked.
        ///
        /// When this register has an architecturally-defined reset value, this
        /// field resets to 1 .
        F OFFSET(6) NUMBITS(1) [
            Unmasked = 0,
            Masked = 1
        ]
    ]
}


pub struct Reg;

impl RegisterReadWrite<u32, DAIF::Register> for Reg {
    sys_coproc_read_raw!(u32, "DAIF");
    sys_coproc_write_raw!(u32, "DAIF");
}

pub static DAIF: Reg = Reg {};
