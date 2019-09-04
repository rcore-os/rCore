//! Raspberry PI 3 Model B/B+

use bcm2837::{addr::bus_to_phys, atags::Atags};

pub mod emmc;
pub mod irq;
pub mod mailbox;
pub mod serial;
pub mod timer;

use crate::drivers::gpu::fb::{self, ColorDepth, ColorFormat, FramebufferInfo, FramebufferResult};

/// Initialize serial port before other initializations.
pub fn init_serial_early() {
    serial::init();
    println!("Hello Raspberry Pi!");
}

/// Initialize raspi3 drivers
pub fn init_driver() {
    if let Ok(fb_info) = probe_fb_info(0, 0, 0) {
        fb::init(fb_info);
    }
    timer::init();
    emmc::init();
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

fn probe_fb_info(width: u32, height: u32, depth: u32) -> FramebufferResult {
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

    let paddr = bus_to_phys(info.bus_addr);
    let vaddr = crate::memory::phys_to_virt(paddr as usize);
    if vaddr == 0 {
        Err(format!(
            "cannot remap memory range [{:#x?}..{:#x?}]",
            paddr,
            paddr + info.screen_size
        ))?;
    }

    let depth = ColorDepth::try_from(info.depth)?;
    let format = match info.depth {
        16 => ColorFormat::RGB565,
        32 => ColorFormat::BGRA8888,
        _ => Err(format!("unsupported color depth {}", info.depth))?,
    };
    Ok(FramebufferInfo {
        xres: info.xres,
        yres: info.yres,
        xres_virtual: info.xres_virtual,
        yres_virtual: info.yres_virtual,
        xoffset: info.xoffset,
        yoffset: info.yoffset,
        depth: depth,
        format: format,
        paddr: paddr as usize,
        vaddr: vaddr,
        screen_size: info.screen_size as usize,
    })
}
