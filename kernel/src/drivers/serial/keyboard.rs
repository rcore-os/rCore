//! Keyboard in x86
use super::super::DRIVERS;
use super::super::IRQ_MANAGER;
use super::{super::SERIAL_DRIVERS, SerialDriver};
use crate::{
    drivers::{DeviceType, Driver},
    sync::SpinNoIrqLock as Mutex,
};
use alloc::string::String;
use alloc::sync::Arc;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, ScancodeSet1};
use uart_16550::SerialPort;
use x86_64::instructions::port::Port;

pub const Keyboard: usize = 1;

struct Keyboard {
    keyboard: Mutex<pc_keyboard::Keyboard<layouts::Us104Key, ScancodeSet1>>,
}

impl Keyboard {
    fn new() -> Keyboard {
        Keyboard {
            keyboard: Mutex::new(pc_keyboard::Keyboard::new(
                layouts::Us104Key,
                ScancodeSet1,
                HandleControl::Ignore,
            )),
        }
    }
}

impl Driver for Keyboard {
    fn try_handle_interrupt(&self, irq: Option<usize>) -> bool {
        let mut keyboard = self.keyboard.lock();
        let mut data_port = Port::<u8>::new(0x60);
        let mut status_port = Port::<u8>::new(0x64);
        // Output buffer status = 1
        if unsafe { status_port.read() } & (1 << 0) != 0 {
            let scancode = unsafe { data_port.read() };
            if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
                if let Some(key) = keyboard.process_keyevent(key_event) {
                    match key {
                        DecodedKey::Unicode(c) => {
                            // at most 4 is needed
                            let mut buffer = [0u8; 4];
                            let res = c.encode_utf8(&mut buffer);
                            for c in res.bytes() {
                                crate::trap::serial(c)
                            }
                        }
                        DecodedKey::RawKey(code) => {
                            let s = match code {
                                KeyCode::ArrowUp => "\u{1b}[A",
                                KeyCode::ArrowDown => "\u{1b}[B",
                                KeyCode::ArrowRight => "\u{1b}[C",
                                KeyCode::ArrowLeft => "\u{1b}[D",
                                _ => "",
                            };
                            for c in s.bytes() {
                                crate::trap::serial(c);
                            }
                        }
                    }
                }
            }
        }
        true
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Serial
    }

    fn get_id(&self) -> String {
        format!("keyboard")
    }
}

impl SerialDriver for Keyboard {
    fn read(&self) -> u8 {
        unimplemented!()
    }

    fn write(&self, data: &[u8]) {
        unimplemented!()
    }
}

pub fn init() {
    let keyboard = Arc::new(Keyboard::new());
    DRIVERS.write().push(keyboard.clone());
    SERIAL_DRIVERS.write().push(keyboard.clone());
    IRQ_MANAGER.write().register_irq(Keyboard, keyboard);
}
