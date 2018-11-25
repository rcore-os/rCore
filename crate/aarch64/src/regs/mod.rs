//! Processor core registers

#[macro_use]
mod macros;

mod cntfrq_el0;
mod cnthctl_el2;
mod cntp_ctl_el0;
mod cntp_tval_el0;
mod cntpct_el0;
mod cntvoff_el2;
mod currentel;
mod daif;
mod elr_el2;
mod hcr_el2;
mod id_aa64mmfr0_el1;
mod mair_el1;
mod mpidr_el1;
mod sctlr_el1;
mod sp;
mod sp_el0;
mod sp_el1;
mod spsel;
mod spsr_el2;
mod tcr_el1;
mod ttbr0_el1;
mod ttbr1_el1;

// Export only the R/W traits and the static reg definitions
pub use register::cpu::*;

pub use self::cntfrq_el0::CNTFRQ_EL0;
pub use self::cnthctl_el2::CNTHCTL_EL2;
pub use self::cntp_ctl_el0::CNTP_CTL_EL0;
pub use self::cntp_tval_el0::CNTP_TVAL_EL0;
pub use self::cntpct_el0::CNTPCT_EL0;
pub use self::cntvoff_el2::CNTVOFF_EL2;
pub use self::currentel::CurrentEL;
pub use self::daif::DAIF;
pub use self::elr_el2::ELR_EL2;
pub use self::hcr_el2::HCR_EL2;
pub use self::id_aa64mmfr0_el1::ID_AA64MMFR0_EL1;
pub use self::mair_el1::{MAIR_EL1, MAIR_ATTR};
pub use self::mpidr_el1::MPIDR_EL1;
pub use self::sctlr_el1::SCTLR_EL1;
pub use self::sp::SP;
pub use self::sp_el0::SP_EL0;
pub use self::sp_el1::SP_EL1;
pub use self::spsel::SPSel;
pub use self::spsr_el2::SPSR_EL2;
pub use self::tcr_el1::TCR_EL1;
pub use self::ttbr0_el1::TTBR0_EL1;
pub use self::ttbr1_el1::TTBR1_EL1;
