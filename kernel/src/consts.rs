#![allow(dead_code)]

pub use crate::arch::consts::*;

pub const MAX_CPU_NUM: usize = 64;
pub const MAX_PROCESS_NUM: usize = 512;

pub const USEC_PER_TICK: usize = 10000;

pub const INFORM_PER_MSEC: usize = 50;

#[cfg(target_arch = "x86_64")]
pub const ARCH: &'static str = "x86_64";
#[cfg(target_arch = "riscv64")]
pub const ARCH: &'static str = "riscv64";
#[cfg(target_arch = "riscv32")]
pub const ARCH: &'static str = "riscv32";
#[cfg(target_arch = "aarch64")]
pub const ARCH: &'static str = "aarch64";

lazy_static! {
    pub static ref SMP_CORES: usize = {
        if let Some(smp_str) = option_env!("SMP") {
            if let Ok(smp) = smp_str.parse() {
                smp
            } else {
                MAX_CPU_NUM
            }
        } else {
            MAX_CPU_NUM
        }
    };
}
