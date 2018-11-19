use ::IO_BASE;
use volatile::{ReadOnly, Volatile};
use interrupt::{Controller, Interrupt};

/// The base address for the ARM system timer registers.
const TIMER_REG_BASE: usize = IO_BASE + 0x3000;

/// System timer registers (ref: peripherals 12.1, page 172)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    CS: Volatile<u32>,
    CLO: ReadOnly<u32>,
    CHI: ReadOnly<u32>,
    COMPARE: [Volatile<u32>; 4],
}

#[repr(u8)]
#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Debug)]
enum SystemTimer {
    Timer0 = 0,
    Timer1 = 1,
    Timer2 = 2,
    Timer3 = 3,
}

/// The Raspberry Pi ARM system timer.
pub struct Timer {
    registers: &'static mut Registers,
}

impl Timer {
    /// Returns a new instance of `Timer`.
    pub fn new() -> Timer {
        Timer {
            registers: unsafe { &mut *(TIMER_REG_BASE as *mut Registers) },
        }
    }

    /// Reads the system timer's counter and returns the 64-bit counter value.
    /// The returned value is the number of elapsed microseconds.
    pub fn read(&self) -> u64 {
        let low = self.registers.CLO.read();
        let high = self.registers.CHI.read();
        ((high as u64) << 32) | (low as u64)
    }

    /// Sets up a match in timer 1 to occur `us` microseconds from now. If
    /// interrupts for timer 1 are enabled and IRQs are unmasked, then a timer
    /// interrupt will be issued in `us` microseconds.
    pub fn tick_in(&mut self, us: u32) {
        let current_low = self.registers.CLO.read();
        let compare = current_low.wrapping_add(us);
        self.registers.COMPARE[SystemTimer::Timer1 as usize].write(compare);
        self.registers.CS.write(1 << (SystemTimer::Timer1 as usize)); // unmask
    }

    /// Initialization timer
    pub fn init(&mut self) {
        Controller::new().enable(Interrupt::Timer1);
    }

    /// Returns `true` if timer interruption is pending. Otherwise, returns `false`.
    pub fn is_pending(&self) -> bool {
        let controller = Controller::new();
        controller.is_pending(Interrupt::Timer1)
    }
}
