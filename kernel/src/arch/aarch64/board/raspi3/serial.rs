use bcm2837::mini_uart::{MiniUart, MiniUartInterruptId};
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;

/// Struct to get a global SerialPort interface
pub struct SerialPort {
    mu: MiniUart,
}

pub trait SerialRead {
    fn receive(&mut self) -> u8;
}

impl SerialPort {
    /// Creates a new instance of `SerialPort`.
    fn new() -> SerialPort {
        SerialPort {
            mu: MiniUart::new(),
        }
    }

    /// Init a newly created SerialPort, can only be called once.
    fn init(&mut self) {
        self.mu.init();
        super::irq::register_irq(super::irq::Interrupt::Aux, handle_serial_irq);
    }

    /// Writes the byte `byte` to the UART device.
    fn write_byte(&mut self, byte: u8) {
        self.mu.write_byte(byte)
    }

    /// Reads a byte from the UART device, blocking until a byte is available.
    fn read_byte(&self) -> u8 {
        self.mu.read_byte()
    }

    // Whether the interrupt `id` is pending.
    fn interrupt_is_pending(&self, id: MiniUartInterruptId) -> bool {
        self.mu.interrupt_is_pending(id)
    }
}

impl SerialRead for SerialPort {
    fn receive(&mut self) -> u8 {
        self.read_byte()
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                // Backspace
                b'\x7f' => {
                    self.write_byte(b'\x08');
                    self.write_byte(b' ');
                    self.write_byte(b'\x08');
                }
                // Return
                b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                }
                // Others
                _ => self.write_byte(byte),
            }
        }
        Ok(())
    }
}

fn handle_serial_irq() {
    let serial = SERIAL_PORT.lock();
    if serial.interrupt_is_pending(MiniUartInterruptId::Recive) {
        crate::trap::serial(serial.read_byte() as char)
    }
}

lazy_static! {
    pub static ref SERIAL_PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new());
}

pub fn init() {
    SERIAL_PORT.lock().init();
}
