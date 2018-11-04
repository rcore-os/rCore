use IO_BASE;
use volatile::{ReadOnly, Volatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

/// Allowed interrupts (ref: peripherals 7.5, page 113)
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Interrupt {
    Timer1 = 1,
    Timer3 = 3,
    Usb = 9,
    Aux = 29,
    Gpio0 = 49,
    Gpio1 = 50,
    Gpio2 = 51,
    Gpio3 = 52,
    Uart = 57,
}

/// Interrupts registers starting from `INT_BASE` (ref: peripherals 7.5, page 112)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    IRQBasicPending: ReadOnly<u32>,
    IRQPending: [ReadOnly<u32>; 2],
    FIQControl: Volatile<u32>,
    EnableIRQ: [Volatile<u32>; 2],
    EnableBasicIRQ: Volatile<u32>,
    DisableIRQ: [Volatile<u32>; 2],
    DisableBasicIRQ: Volatile<u32>,
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers,
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    pub fn new() -> Controller {
        Controller {
            registers: unsafe { &mut *(INT_BASE as *mut Registers) },
        }
    }

    /// Enables the interrupt `int`.
    pub fn enable(&mut self, int: Interrupt) {
        self.registers.EnableIRQ[int as usize / 32].write(1 << (int as usize) % 32);
    }

    /// Disables the interrupt `int`.
    pub fn disable(&mut self, int: Interrupt) {
        self.registers.DisableIRQ[int as usize / 32].write(1 << (int as usize) % 32);
    }

    /// Returns `true` if `int` is pending. Otherwise, returns `false`.
    pub fn is_pending(&self, int: Interrupt) -> bool {
        self.registers.IRQPending[int as usize / 32].read() & (1 << (int as usize) % 32) != 0
    }
}
