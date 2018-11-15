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

//! Translation Control Register - EL1
//!
//! The control register for stage 1 of the EL1&0 translation regime.

use register::cpu::RegisterReadWrite;

register_bitfields! {u64,
    TCR_EL1 [
        /// Top Byte ignored - indicates whether the top byte of an address is
        /// used for address match for the TTBR0_EL1 region, or ignored and used
        /// for tagged addresses. Defined values are:
        ///
        /// 0 Top Byte used in the address calculation.
        /// 1 Top Byte ignored in the address calculation.
        TBI0  OFFSET(37) NUMBITS(1) [
            Used = 0,
            Ignored = 1
        ],

        /// Intermediate Physical Address Size.
        ///
        /// 000 32 bits, 4GiB.
        /// 001 36 bits, 64GiB.
        /// 010 40 bits, 1TiB.
        /// 011 42 bits, 4TiB.
        /// 100 44 bits, 16TiB.
        /// 101 48 bits, 256TiB.
        /// 110 52 bits, 4PiB
        ///
        /// Other values are reserved.
        ///
        /// The reserved values behave in the same way as the 101 or 110
        /// encoding, but software must not rely on this property as the
        /// behavior of the reserved values might change in a future revision of
        /// the architecture.
        ///
        /// The value 110 is permitted only if ARMv8.2-LPA is implemented and
        /// the translation granule size is 64KiB.
        ///
        /// In an implementation that supports 52-bit PAs, if the value of this
        /// field is not 110 , then bits[51:48] of every translation table base
        /// address for the stage of translation controlled by TCR_EL1 are 0000
        /// .
        IPS   OFFSET(32) NUMBITS(3) [
            Bits_32 = 0b000,
            Bits_36 = 0b001,
            Bits_40 = 0b010,
            Bits_42 = 0b011,
            Bits_44 = 0b100,
            Bits_48 = 0b101,
            Bits_52 = 0b110
        ],

        /// Granule size for the TTBR0_EL1.
        ///
        /// 00 4KiB
        /// 01 64KiB
        /// 10 16KiB
        ///
        /// Other values are reserved.
        ///
        /// If the value is programmed to either a reserved value, or a size
        /// that has not been implemented, then the hardware will treat the
        /// field as if it has been programmed to an IMPLEMENTATION DEFINED
        /// choice of the sizes that has been implemented for all purposes other
        /// than the value read back from this register.
        ///
        /// It is IMPLEMENTATION DEFINED whether the value read back is the
        /// value programmed or the value that corresponds to the size chosen.
        TG0   OFFSET(14) NUMBITS(2) [
            KiB_4 = 0b00,
            KiB_16 = 0b10,
            KiB_64 = 0b01
        ],

        /// Shareability attribute for memory associated with translation table
        /// walks using TTBR0_EL1.
        ///
        /// 00 Non-shareable
        /// 10 Outer Shareable
        /// 11 Inner Shareable
        ///
        /// Other values are reserved.
        SH0   OFFSET(12) NUMBITS(2) [
            None = 0b00,
            Outer = 0b10,
            Inner = 0b11
        ],

        /// Outer cacheability attribute for memory associated with translation
        /// table walks using TTBR0_EL1.
        ///
        /// 00 Normal memory, Outer Non-cacheable
        ///
        /// 01 Normal memory, Outer Write-Back Read-Allocate Write-Allocate
        ///    Cacheable
        ///
        /// 10 Normal memory, Outer Write-Through Read-Allocate No
        ///    Write-Allocate Cacheable
        ///
        /// 11 Normal memory, Outer Write-Back Read-Allocate No Write-Allocate
        ///    Cacheable
        ORGN0 OFFSET(10) NUMBITS(2) [
            NonCacheable = 0b00,
            WriteBack_ReadAlloc_WriteAlloc_Cacheable = 0b01,
            WriteThrough_ReadAlloc_NoWriteAlloc_Cacheable = 0b10,
            WriteBack_ReadAlloc_NoWriteAlloc_Cacheable = 0b11
        ],

        /// Inner cacheability attribute for memory associated with translation
        /// table walks using TTBR0_EL1.
        ///
        /// 00 Normal memory, Inner Non-cacheable
        ///
        /// 01 Normal memory, Inner Write-Back Read-Allocate Write-Allocate
        ///    Cacheable
        ///
        /// 10 Normal memory, Inner Write-Through Read-Allocate No
        ///    Write-Allocate Cacheable
        ///
        /// 11 Normal memory, Inner Write-Back Read-Allocate No Write-Allocate
        ///    Cacheable
        IRGN0 OFFSET(8) NUMBITS(2) [
            NonCacheable = 0b00,
            WriteBack_ReadAlloc_WriteAlloc_Cacheable = 0b01,
            WriteThrough_ReadAlloc_NoWriteAlloc_Cacheable = 0b10,
            WriteBack_ReadAlloc_NoWriteAlloc_Cacheable = 0b11
        ],

        /// Translation table walk disable for translations using
        /// TTBR0_EL1. This bit controls whether a translation table walk is
        /// performed on a TLB miss, for an address that is translated using
        /// TTBR0_EL1. The encoding of this bit is:
        ///
        /// 0 Perform translation table walks using TTBR0_EL1.
        ///
        /// 1 A TLB miss on an address that is translated using TTBR0_EL1
        ///   generates a Translation fault. No translation table walk is
        ///   performed.
        EPD0  OFFSET(7) NUMBITS(1) [
            EnableTTBR0Walks = 0,
            DisableTTBR0Walks = 1
        ],

        /// The size offset of the memory region addressed by TTBR0_EL1. The
        /// region size is 2^(64-T0SZ) bytes.
        ///
        /// The maximum and minimum possible values for T0SZ depend on the level
        /// of translation table and the memory translation granule size, as
        /// described in the AArch64 Virtual Memory System Architecture chapter.
        T0SZ  OFFSET(0) NUMBITS(6) []
    ]
}

pub struct Reg;

impl RegisterReadWrite<u64, TCR_EL1::Register> for Reg {
    sys_coproc_read_raw!(u64, "TCR_EL1");
    sys_coproc_write_raw!(u64, "TCR_EL1");
}

pub static TCR_EL1: Reg = Reg {};
