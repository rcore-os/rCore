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
        // TODO: Macrofy this

        /// Attribute 7
        Attr7_HIGH OFFSET(60) NUMBITS(4) [],
        Attr7_LOW_DEVICE OFFSET(56) NUMBITS(4) [],
        Attr7_LOW_MEMORY OFFSET(56) NUMBITS(4) [],

        /// Attribute 6
        Attr6_HIGH OFFSET(52) NUMBITS(4) [],
        Attr6_LOW_DEVICE OFFSET(48) NUMBITS(4) [],
        Attr6_LOW_MEMORY OFFSET(48) NUMBITS(4) [],

        /// Attribute 5
        Attr5_HIGH OFFSET(44) NUMBITS(4) [],
        Attr5_LOW_DEVICE OFFSET(40) NUMBITS(4) [],
        Attr5_LOW_MEMORY OFFSET(40) NUMBITS(4) [],

        /// Attribute 4
        Attr4_HIGH OFFSET(36) NUMBITS(4) [],
        Attr4_LOW_DEVICE OFFSET(32) NUMBITS(4) [],
        Attr4_LOW_MEMORY OFFSET(32) NUMBITS(4) [],

        /// Attribute 3
        Attr3_HIGH OFFSET(28) NUMBITS(4) [],
        Attr3_LOW_DEVICE OFFSET(24) NUMBITS(4) [],
        Attr3_LOW_MEMORY OFFSET(24) NUMBITS(4) [],

        /// Attribute 2
        Attr2_HIGH OFFSET(20) NUMBITS(4) [
            Device = 0b0000,
            Memory_OuterNonCacheable = 0b0100,
            Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ],
        Attr2_LOW_DEVICE OFFSET(16) NUMBITS(4) [
            Device_nGnRE = 0b0100
        ],
        Attr2_LOW_MEMORY OFFSET(16) NUMBITS(4) [
            InnerNonCacheable = 0b0100,
            InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ],

        /// Attribute 1
        Attr1_HIGH OFFSET(12) NUMBITS(4) [
            Device = 0b0000,
            Memory_OuterNonCacheable = 0b0100,
            Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ],
        Attr1_LOW_DEVICE OFFSET(8) NUMBITS(4) [
            Device_nGnRE = 0b0100
        ],
        Attr1_LOW_MEMORY OFFSET(8) NUMBITS(4) [
            InnerNonCacheable = 0b0100,
            InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ],

        /// Attribute 0
        Attr0_HIGH OFFSET(4) NUMBITS(4) [
            Device = 0b0000,
            Memory_OuterNonCacheable = 0b0100,
            Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc = 0b1111
        ],
        Attr0_LOW_DEVICE OFFSET(0) NUMBITS(4) [
            Device_nGnRE = 0b0100
        ],
        Attr0_LOW_MEMORY OFFSET(0) NUMBITS(4) [
            InnerNonCacheable = 0b0100,
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
