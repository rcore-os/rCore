use IO_BASE;
use gpio::{Function, Gpio};
use volatile::{ReadOnly, Volatile};

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

/// MU registers starting from `AUX_ENABLES` (ref: peripherals 2.1, page 8)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    AUX_MU_IO_REG: Volatile<u8>,
    __r0: [u8; 3],
    AUX_MU_IER_REG: Volatile<u8>,
    __r1: [u8; 3],
    AUX_MU_IIR_REG: Volatile<u8>,
    __r2: [u8; 3],
    AUX_MU_LCR_REG: Volatile<u8>,
    __r3: [u8; 3],
    AUX_MU_MCR_REG: Volatile<u8>,
    __r4: [u8; 3],
    AUX_MU_LSR_REG: ReadOnly<u8>,
    __r5: [u8; 3],
    AUX_MU_MSR_REG: ReadOnly<u8>,
    __r6: [u8; 3],
    AUX_MU_SCRATCH: Volatile<u8>,
    __r7: [u8; 3],
    AUX_MU_CNTL_REG: Volatile<u8>,
    __r8: [u8; 3],
    AUX_MU_STAT_REG: ReadOnly<u32>,
    AUX_MU_BAUD_REG: Volatile<u16>,
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<u32>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new() -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*AUX_ENABLES).write(1);
            &mut *(MU_REG_BASE as *mut Registers)
        };

        Gpio::new(14).into_alt(Function::Alt5).set_gpio_pd(0);
        Gpio::new(15).into_alt(Function::Alt5).set_gpio_pd(0);

        registers.AUX_MU_CNTL_REG.write(0); // Disable auto flow control and disable receiver and transmitter (for now)
        registers.AUX_MU_IER_REG.write(0); // Disable receive and transmit interrupts
        registers.AUX_MU_LCR_REG.write(3); // Enable 8 bit mode
        registers.AUX_MU_MCR_REG.write(0); // Set RTS line to be always high
        registers.AUX_MU_BAUD_REG.write(270); // Set baud rate to 115200

        registers.AUX_MU_CNTL_REG.write(3); // Finally, enable transmitter and receiver

        MiniUart {
            registers: registers,
            timeout: None,
        }
    }

    /// Set the read timeout to `milliseconds` milliseconds.
    pub fn set_read_timeout(&mut self, milliseconds: u32) {
        self.timeout = Some(milliseconds)
    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while self.registers.AUX_MU_LSR_REG.read() & (LsrStatus::TxAvailable as u8) == 0 {}
        self.registers.AUX_MU_IO_REG.write(byte);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        self.registers.AUX_MU_LSR_REG.read() & (LsrStatus::DataReady as u8) != 0
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        unimplemented!()
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        while !self.has_byte() {}
        self.registers.AUX_MU_IO_REG.read()
    }
}
