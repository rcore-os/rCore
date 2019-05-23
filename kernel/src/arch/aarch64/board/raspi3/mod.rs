//! Raspberry PI 3 Model B/B+

use bcm2837::atags::Atags;

#[path = "../../../../drivers/gpu/fb.rs"]
pub mod fb;
pub mod irq;
pub mod mailbox;
pub mod serial;
pub mod timer;

use fb::{ColorConfig, FramebufferResult};

pub const IO_REMAP_BASE: usize = bcm2837::consts::IO_BASE;
pub const IO_REMAP_END: usize = bcm2837::consts::KERNEL_OFFSET + 0x4000_1000;

/// Initialize serial port before other initializations.
pub fn init_serial_early() {
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

pub fn probe_fb_info(width: u32, height: u32, depth: u32) -> FramebufferResult {
    let (width, height) = if width == 0 || height == 0 {
        mailbox::framebuffer_get_physical_size()?
    } else {
        (width, height)
    };

    let depth = if depth == 0 {
        mailbox::framebuffer_get_depth()?
    } else {
        depth
    };

    let info = mailbox::framebuffer_alloc(width, height, depth)?;

    if info.bus_addr == 0 || info.screen_size == 0 {
        Err(format!("mailbox call returned an invalid address/size"))?;
    }
    if info.pitch == 0 || info.pitch != info.xres * info.depth / 8 {
        Err(format!(
            "mailbox call returned an invalid pitch value {}",
            info.pitch
        ))?;
    }

    let paddr = info.bus_addr & !0xC0000000;
    let vaddr = crate::memory::phys_to_virt(paddr as usize);
    if vaddr == 0 {
        Err(format!(
            "cannot remap memory range [{:#x?}..{:#x?}]",
            paddr,
            paddr + info.screen_size
        ))?;
    }

    let color_config = match info.depth {
        16 => ColorConfig::RGB565,
        32 => ColorConfig::BGRA8888,
        _ => Err(format!("unsupported color depth {}", info.depth))?,
    };
    Ok((info, color_config, vaddr))
}
