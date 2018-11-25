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

//! Memory Attribute Indirection Register - EL1
//!
//! Provides the memory attribute encodings corresponding to the possible
//! AttrIndx values in a Long-descriptor format translation table entry for
//! stage 1 translations at EL1.

use register::cpu::RegisterReadWrite;

register_bitfields! {u64,
    MAIR_EL1 [
        /// Attribute 7
        Attr7 OFFSET(56) NUMBITS(8) [],

        /// Attribute 6
        Attr6 OFFSET(48) NUMBITS(8) [],

        /// Attribute 5
        Attr5 OFFSET(40) NUMBITS(8) [],

        /// Attribute 4
        Attr4 OFFSET(32) NUMBITS(8) [],

        /// Attribute 3
        Attr3 OFFSET(24) NUMBITS(8) [],

        /// Attribute 2
        Attr2 OFFSET(16) NUMBITS(8) [],

        /// Attribute 1
        Attr1 OFFSET(8) NUMBITS(8) [],

        /// Attribute 0
        Attr0 OFFSET(0) NUMBITS(8) []
    ]
}

register_bitfields! {u64,
    MAIR_ATTR [
        Attr_HIGH OFFSET(4) NUMBITS(4) [
            Device = 0b0000,
            Memory_OuterNonCacheable = 0b0100,
            Memory_OuterWriteThrough_NonTransient_ReadAlloc_WriteAlloc = 0b1011,
            Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ],
        Attr_LOW_DEVICE OFFSET(0) NUMBITS(4) [
            Device_nGnRnE = 0b0000,
            Device_nGnRE  = 0b0100,
            Device_nGRE   = 0b1000,
            Device_GRE    = 0b1100
        ],
        Attr_LOW_MEMORY OFFSET(0) NUMBITS(4) [
            InnerNonCacheable = 0b0100,
            InnerWriteThrough_NonTransient_ReadAlloc_WriteAlloc = 0b1011,
            InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ]
    ]
}

pub struct Reg;

impl RegisterReadWrite<u64, MAIR_EL1::Register> for Reg {
    sys_coproc_read_raw!(u64, "MAIR_EL1");
    sys_coproc_write_raw!(u64, "MAIR_EL1");
}

pub static MAIR_EL1: Reg = Reg {};
