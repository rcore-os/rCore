//!Memory region attributes (D4.5, page 2174)

use super::{PageTableAttribute, MEMORY_ATTRIBUTE};
use regs::*;

pub trait MairType {
    const INDEX: u64;

    #[inline]
    fn config_value() -> u64;

    #[inline]
    fn attr_value() -> PageTableAttribute;
}

pub enum MairDevice {}
pub enum MairNormal {}
pub enum MairNormalNonCacheable {}

impl MairType for MairDevice {
    const INDEX: u64 = 0;

    #[inline]
    fn config_value() -> u64 {
        (MAIR_ATTR::Attr_HIGH::Device + MAIR_ATTR::Attr_LOW_DEVICE::Device_nGnRE).value
    }

    #[inline]
    fn attr_value() -> PageTableAttribute {
        MEMORY_ATTRIBUTE::SH::OuterShareable + MEMORY_ATTRIBUTE::AttrIndx.val(Self::INDEX)
    }
}

impl MairType for MairNormal {
    const INDEX: u64 = 1;

    #[inline]
    fn config_value() -> u64 {
        (MAIR_ATTR::Attr_HIGH::Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc
            + MAIR_ATTR::Attr_LOW_MEMORY::InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc)
            .value
    }

    #[inline]
    fn attr_value() -> PageTableAttribute {
        MEMORY_ATTRIBUTE::SH::InnerShareable + MEMORY_ATTRIBUTE::AttrIndx.val(Self::INDEX)
    }
}

impl MairType for MairNormalNonCacheable {
    const INDEX: u64 = 2;

    #[inline]
    fn config_value() -> u64 {
        (MAIR_ATTR::Attr_HIGH::Memory_OuterNonCacheable
            + MAIR_ATTR::Attr_LOW_MEMORY::InnerNonCacheable)
            .value
    }

    #[inline]
    fn attr_value() -> PageTableAttribute {
        MEMORY_ATTRIBUTE::SH::OuterShareable + MEMORY_ATTRIBUTE::AttrIndx.val(Self::INDEX)
    }
}
