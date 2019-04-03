//! stub frame buffer driver for malta board

use spin::Mutex;

lazy_static! {
    pub static ref FRAME_BUFFER: Mutex<Option<Framebuffer>> = Mutex::new(None);
}

pub struct Framebuffer {}

pub struct FramebufferInfo {}

pub enum ColorDepth {}