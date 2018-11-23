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

// Borrow implementations from the pending upstream ACLE implementation until it
// is merged. Afterwards, we'll probably just reexport them, hoping that the API
// doesn't change.
//
// https://github.com/rust-lang-nursery/stdsimd/pull/557

mod sealed {
    pub trait Dmb {
        unsafe fn __dmb(&self);
    }

    pub trait Dsb {
        unsafe fn __dsb(&self);
    }

    pub trait Isb {
        unsafe fn __isb(&self);
    }
}

macro_rules! dmb_dsb {
    ($A:ident) => {
        impl sealed::Dmb for $A {
            #[inline(always)]
            unsafe fn __dmb(&self) {
                asm!(concat!("DMB ", stringify!($A)) : : : "memory" : "volatile")
            }
        }
        impl sealed::Dsb for $A {
            #[inline(always)]
            unsafe fn __dsb(&self) {
                asm!(concat!("DSB ", stringify!($A)) : : : "memory" : "volatile")
            }
        }
    };
}

pub struct SY;
pub struct ISH;
pub struct ISHST;

dmb_dsb!(SY);
dmb_dsb!(ISH);
dmb_dsb!(ISHST);

impl sealed::Isb for SY {
    #[inline(always)]
    unsafe fn __isb(&self) {
        asm!("ISB SY" : : : "memory" : "volatile")
    }
}

#[inline(always)]
pub unsafe fn dmb<A>(arg: A)
where
    A: sealed::Dmb,
{
    arg.__dmb()
}

#[inline(always)]
pub unsafe fn dsb<A>(arg: A)
where
    A: sealed::Dsb,
{
    arg.__dsb()
}

#[inline(always)]
pub unsafe fn isb<A>(arg: A)
where
    A: sealed::Isb,
{
    arg.__isb()
}
