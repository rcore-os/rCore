use crate::IO_BASE;
use volatile::{ReadOnly, Volatile};

const INT_BASE: usize = IO_BASE + 0xB000 + 0x200;

/// Allowed interrupts (ref: peripherals 7.5, page 113)
#[repr(u8)]
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

/// Pending interrupts
pub struct PendingInterrupts(u64);

impl Iterator for PendingInterrupts {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let int = self.0.trailing_zeros();
        if int < 64 {
            self.0 &= !(1 << int);
            Some(int as usize)
        } else {
            None
        }
    }
}

/// An interrupt controller. Used to enable and disable interrupts as well as to
/// check if an interrupt is pending.
pub struct Controller {
    registers: &'static mut Registers,
}

impl Controller {
    /// Returns a new handle to the interrupt controller.
    #[inline]
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

    /// Return all pending interrupts.
    pub fn pending_interrupts(&self) -> PendingInterrupts {
        let irq1 = self.registers.IRQPending[0].read() as u64;
        let irq2 = self.registers.IRQPending[1].read() as u64;
        PendingInterrupts((irq2 << 32) | irq1)
    }
}
