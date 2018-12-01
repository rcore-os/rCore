use bcm2837::mini_uart::MiniUart;
use core::fmt;
use spin::Mutex;

/// Struct to get a global SerialPort interface
pub struct SerialPort {
    mu: Option<MiniUart>,
}

pub trait SerialRead {
    fn receive(&mut self) -> u8;
}

impl SerialPort {
    /// Creates a new instance of `SerialPort`.
    const fn new() -> SerialPort {
        SerialPort { mu: None }
    }

    /// Init a newly created SerialPort, can only be called once.
    pub fn init(&mut self) {
        assert_has_not_been_called!("SerialPort::init must be called only once");
        self.mu = Some(MiniUart::new());
    }

    /// Writes the byte `byte` to the UART device.
    pub fn write_byte(&mut self, byte: u8) {
        match &mut self.mu {
            Some(mu) => mu.write_byte(byte),
            None => panic!("SerialPort is not initialized"),
        }
    }

    /// Reads a byte from the UART device, blocking until a byte is available.
    pub fn read_byte(&mut self) -> u8 {
        match &mut self.mu {
            Some(mu) => return mu.read_byte(),
            None => panic!("SerialPort is not initialized"),
        }
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

pub static SERIAL_PORT: Mutex<SerialPort> = Mutex::new(SerialPort::new());
