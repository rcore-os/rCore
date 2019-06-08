// Copied from musl ldso.
pub mod aarch64;
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use self::x86_64::*;

#[cfg(target_arch = "aarch64")]
pub use self::aarch64::*;
