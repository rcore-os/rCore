use spin::Mutex;
use x86_64::instructions::port::Port;
use pc_keyboard::{Keyboard, ScancodeSet1, DecodedKey, layouts, HandleControl};
use lazy_static::lazy_static;

pub fn init() {
    use crate::arch::interrupt::consts;
    use crate::arch::interrupt::enable_irq;
	enable_irq(consts::Keyboard);
}

/// Receive character from keyboard
/// Should be called on every interrupt
pub fn receive() -> Option<char> {
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore));
    }

    let mut keyboard = KEYBOARD.lock();
    let data_port = Port::<u8>::new(0x60);
    let status_port = Port::<u8>::new(0x64);

    // Output buffer status = 1
    if unsafe { status_port.read() } & (1 << 0) != 0 {
        let scancode = unsafe { data_port.read() };
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => return Some(character),
                    DecodedKey::RawKey(_key) => {}, // TODO: handle RawKey from keyboard
                }
            }
        }
    }
    None
}