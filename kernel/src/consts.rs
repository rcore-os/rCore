#![allow(dead_code)]

pub use crate::arch::consts::*;

pub const MAX_CPU_NUM: usize = 64;
pub const MAX_PROCESS_NUM: usize = 128;

pub const USEC_PER_TICK: usize = 10000;

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
