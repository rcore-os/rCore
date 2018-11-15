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

//! Counter-timer Physical Timer Control register - EL0
//!
//! Control register for the EL1 physical timer.

use register::cpu::RegisterReadWrite;

register_bitfields! {u32,
    CNTP_CTL_EL0 [
        /// The status of the timer. This bit indicates whether the timer
        /// condition is met:
        ///
        /// 0 Timer condition is not met.
        /// 1 Timer condition is met.
        ///
        /// When the value of the ENABLE bit is 1, ISTATUS indicates whether the
        /// timer condition is met. ISTATUS takes no account of the value of the
        /// IMASK bit. If the value of ISTATUS is 1 and the value of IMASK is 0
        /// then the timer interrupt is asserted.
        ///
        /// When the value of the ENABLE bit is 0, the ISTATUS field is UNKNOWN.
        ///
        /// This bit is read-only.
        ISTATUS OFFSET(2) NUMBITS(1) [],

        /// Timer interrupt mask bit. Permitted values are:
        ///
        /// 0 Timer interrupt is not masked by the IMASK bit.
        /// 1 Timer interrupt is masked by the IMASK bit.
        IMASK   OFFSET(1) NUMBITS(1) [],

        /// Enables the timer. Permitted values are:
        ///
        /// 0 Timer disabled.
        /// 1 Timer enabled.
        ENABLE  OFFSET(0) NUMBITS(1) []
    ]
}

pub struct Reg;

impl RegisterReadWrite<u32, CNTP_CTL_EL0::Register> for Reg {
    sys_coproc_read_raw!(u32, "CNTP_CTL_EL0");
    sys_coproc_write_raw!(u32, "CNTP_CTL_EL0");
}

pub static CNTP_CTL_EL0: Reg = Reg {};
