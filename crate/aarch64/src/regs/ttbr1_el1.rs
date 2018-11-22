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

//! Translation Table Base Register 1 - EL1
//!
//! Holds the base address of the translation table for the initial lookup for
//! stage 1 of the translation of an address from the upper VA range in the
//! EL1&0 translation regime, and other information for this translation regime.

use register::cpu::RegisterReadWrite;

register_bitfields! {u64,
    TTBR1_EL1 [
        /// An ASID for the translation table base address. The TCR_EL1.A1 field
        /// selects either TTBR0_EL1.ASID or TTBR1_EL1.ASID.
        ///
        /// If the implementation has only 8 bits of ASID, then the upper 8 bits
        /// of this field are RES 0.
        ASID  OFFSET(48) NUMBITS(16) [],

        /// Translation table base address
        BADDR OFFSET(1) NUMBITS(47) [],

        /// Common not Private
        CnP   OFFSET(0) NUMBITS(1) []
    ]
}

pub struct Reg;

impl RegisterReadWrite<u64, TTBR1_EL1::Register> for Reg {
    sys_coproc_read_raw!(u64, "TTBR1_EL1");
    sys_coproc_write_raw!(u64, "TTBR1_EL1");
}

impl Reg {
    #[inline]
    pub fn get_baddr(&self) -> u64 {
        self.read(TTBR1_EL1::BADDR) << 1
    }

    #[inline]
    pub fn set_baddr(&self, addr: u64) {
        self.write(TTBR1_EL1::BADDR.val(addr >> 1));
    }
}

pub static TTBR1_EL1: Reg = Reg {};
