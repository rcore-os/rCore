extern crate pc_keyboard;

use spin::Mutex;
use x86_64::instructions::port::Port;
use self::pc_keyboard::{Keyboard, ScancodeSet1, DecodedKey, layouts};

pub fn init() {
	assert_has_not_been_called!("keyboard::init must be called only once");

	use arch::interrupt::consts::*;
	use arch::interrupt::enable_irq;
	enable_irq(IRQ_KBD);
}

/// Receive character from keyboard
/// Should be called on every interrupt
pub fn receive() -> Option<char> {
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(layouts::Us104Key, ScancodeSet1));
    }

    let mut keyboard = KEYBOARD.lock();
    let port = Port::<u8>::new(0x60);

    let scancode = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => return Some(character),
                DecodedKey::RawKey(key) => {}, // TODO: handle RawKey from keyboard
            }
        }
    }
    None
}