use core::fmt::{self, Write};
use spin::Mutex;
use syscall::io::{Io, Pio, Mmio, ReadOnly};

pub static SERIAL: Mutex<Serial> = Mutex::new(Serial::new(0x3F8));

#[allow(dead_code)]
pub struct SerialPort<T: Io<Value = u8>> {
    /// Data register, read to receive, write to send
    data: T,
    /// Interrupt enable
    int_en: T,
    /// FIFO control
    fifo_ctrl: T,
    /// Line control
    line_ctrl: T,
    /// Modem control
    modem_ctrl: T,
    /// Line status
    line_sts: ReadOnly<T>,
    /// Modem status
    modem_sts: ReadOnly<T>,
}

type Serial = SerialPort<Pio<u8>>;

impl SerialPort<Pio<u8>> {
    pub const fn new(base: u16) -> SerialPort<Pio<u8>> {
        SerialPort {
            data: Pio::new(base),
            int_en: Pio::new(base + 1),
            fifo_ctrl: Pio::new(base + 2),
            line_ctrl: Pio::new(base + 3),
            modem_ctrl: Pio::new(base + 4),
            line_sts: ReadOnly::new(Pio::new(base + 5)),
            modem_sts: ReadOnly::new(Pio::new(base + 6))
        }
    }
}

impl SerialPort<Mmio<u8>> {
    pub fn new(_base: usize) -> SerialPort<Mmio<u8>> {
        SerialPort {
            data: Mmio::new(),
            int_en: Mmio::new(),
            fifo_ctrl: Mmio::new(),
            line_ctrl: Mmio::new(),
            modem_ctrl: Mmio::new(),
            line_sts: ReadOnly::new(Mmio::new()),
            modem_sts: ReadOnly::new(Mmio::new())
        }
    }
}

impl<T: Io<Value = u8>> SerialPort<T> {
    pub fn init(&mut self) {
        //TODO: Cleanup
        self.int_en.write(0x00);
        self.line_ctrl.write(0x80);
        self.data.write(0x03);
        self.int_en.write(0x00);
        self.line_ctrl.write(0x03);
        self.fifo_ctrl.write(0xC7);
        self.modem_ctrl.write(0x0B);
        self.int_en.write(0x01);
    }

    fn line_sts(&self) -> LineStsFlags {
        LineStsFlags::from_bits_truncate(self.line_sts.read())
    }

    pub fn receive(&mut self) {
        while self.line_sts().contains(INPUT_FULL) {
            let data = self.data.read();
            write!(self, "serial receive {}", data);
            // TODO handle received data
        }
    }

    fn wait(&self) {
        while ! self.line_sts().contains(OUTPUT_EMPTY) {}
    }

    pub fn send(&mut self, data: u8) {
        match data {
            8 | 0x7F => {
                self.wait();
                self.data.write(8);
                self.wait();
                self.data.write(b' ');
                self.wait();
                self.data.write(8);
            },
            _ => {
                self.wait();
                self.data.write(data);
            }
        }
    }
}

impl<T: Io<Value = u8>> Write for SerialPort<T> {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for byte in s.bytes() {
            self.send(byte);
        }
        Ok(())
    }
}

bitflags! {
    /// Interrupt enable flags
    flags IntEnFlags: u8 {
        const RECEIVED = 1,
        const SENT = 1 << 1,
        const ERRORED = 1 << 2,
        const STATUS_CHANGE = 1 << 3,
        // 4 to 7 are unused
    }
}

bitflags! {
    /// Line status flags
    flags LineStsFlags: u8 {
        const INPUT_FULL = 1,
        // 1 to 4 unknown
        const OUTPUT_EMPTY = 1 << 5,
        // 6 and 7 unknown
    }
}