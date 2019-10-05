//! Framebuffer console display driver

use super::gpu::fb::{Framebuffer, FRAME_BUFFER};
use rcore_console::{Console, ConsoleOnGraphic};
use spin::Mutex;

// Console -> TextBuffer -> FrameBuffer
type RCoreConsole = ConsoleOnGraphic<Framebuffer>;

pub static CONSOLE: Mutex<Option<RCoreConsole>> = Mutex::new(None);

/// Initialize console driver
pub fn init() {
    if cfg!(feature = "consolegraphic") {
        if let Some(fb) = FRAME_BUFFER.lock().take() {
            // FIXME: now take FrameBuffer out of global variable, then move into Console
            let console = Console::on_frame_buffer(fb.fb_info.xres, fb.fb_info.yres, fb);
            *CONSOLE.lock() = Some(console);
            info!("console: init end");
        } else {
            warn!("console: init failed");
        }
    }
}
