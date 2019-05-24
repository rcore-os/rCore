#[path = "../../../drivers/gpu/fb.rs"]
pub mod fb;

use fb::{ColorConfig, FramebufferInfo, FramebufferResult, FRAME_BUFFER};
use crate::consts::KERNEL_OFFSET;

pub fn init_driver() {
    #[cfg(not(feature = "nographic"))]
    fb::init();
}

pub fn probe_fb_info(width : u32, height : u32, depth : u32) -> FramebufferResult {
    let fb_info = FramebufferInfo {
        xres: 1024,
        yres: 768,
        xres_virtual: 1024,
        yres_virtual: 768,
        xoffset: 0,
        yoffset: 0,
        depth: 24,
        pitch: 1024, // TOKNOW
        bus_addr: 0xfd00_0000,
        screen_size: 1024 * 768 * 3,
    };
    Ok((fb_info, fb::ColorConfig::BGRA8888, KERNEL_OFFSET + 0xf000_0000))
}
