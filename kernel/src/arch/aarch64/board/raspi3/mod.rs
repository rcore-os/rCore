//! Raspberry PI 3 Model B/B+

use once::*;
use bcm2837::atags::Atags;

pub mod fb;
pub mod irq;
pub mod timer;
pub mod serial;
pub mod mailbox;

pub const IO_REMAP_BASE: usize = bcm2837::consts::IO_BASE;
pub const IO_REMAP_END: usize = bcm2837::consts::KERNEL_OFFSET + 0x4000_1000;

/// Initialize serial port before other initializations.
pub fn init_serial_early() {
    assert_has_not_been_called!("board::init must be called only once");

    serial::init();

    println!("Hello Raspberry Pi!");
}

/// Initialize raspi3 drivers
pub fn init_driver() {
    #[cfg(not(feature = "nographic"))]
    fb::init();
    timer::init();
}

/// Returns the (start address, end address) of the physical memory on this
/// system if it can be determined. If it cannot, `None` is returned.
///
/// This function is expected to return `Some` under all normal cirumstances.
pub fn probe_memory() -> Option<(usize, usize)> {
    let mut atags: Atags = Atags::get();
    while let Some(atag) = atags.next() {
        if let Some(mem) = atag.mem() {
            return Some((mem.start as usize, (mem.start + mem.size) as usize));
        }
    }
    None
}
