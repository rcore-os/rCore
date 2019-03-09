mod atag;
mod raw;

use super::consts::KERNEL_OFFSET;
pub use self::atag::*;
pub use self::raw::{Cmd, Core, Mem};

/// The address at which the firmware loads the ATAGS.
const ATAG_BASE: usize = KERNEL_OFFSET + 0x100;

/// An iterator over the ATAGS on this system.
pub struct Atags {
    ptr: &'static raw::Atag,
}

impl Atags {
    /// Returns an instance of `Atags`, an iterator over ATAGS on this system.
    pub fn get() -> Atags {
        Atags {
            ptr: unsafe { &*(ATAG_BASE as *const raw::Atag) },
        }
    }
}

impl Iterator for Atags {
    type Item = Atag;

    /// Iterate over Atags.  Returns a valid Atag until the iterator hits the
    /// Atag::None.
    fn next(&mut self) -> Option<Atag> {
        let cur = self.ptr;
        match cur.next() {
            Some(next) => {
                let result = Some(Atag::from(cur));
                self.ptr = next;
                result
            }
            None => None,
        }
    }
}
