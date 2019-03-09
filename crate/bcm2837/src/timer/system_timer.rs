use super::BasicTimer;
use crate::consts::IO_BASE;
use crate::interrupt::{Controller, Interrupt};
use volatile::{ReadOnly, Volatile};

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
enum SystemTimerId {
    Timer0 = 0,
    Timer1 = 1,
    Timer2 = 2,
    Timer3 = 3,
}

/// The Raspberry Pi ARM system timer.
pub struct SystemTimer {
    registers: &'static mut Registers,
}

impl BasicTimer for SystemTimer {
    fn new() -> Self {
        SystemTimer {
            registers: unsafe { &mut *(TIMER_REG_BASE as *mut Registers) },
        }
    }

    fn init(&mut self) {
        Controller::new().enable(Interrupt::Timer1);
    }

    fn read(&self) -> u64 {
        let low = self.registers.CLO.read();
        let high = self.registers.CHI.read();
        ((high as u64) << 32) | (low as u64)
    }

    fn tick_in(&mut self, us: u32) {
        let current_low = self.registers.CLO.read();
        let compare = current_low.wrapping_add(us);
        self.registers.COMPARE[SystemTimerId::Timer1 as usize].write(compare);
        self.registers.CS.write(1 << (SystemTimerId::Timer1 as usize)); // unmask
    }

    fn is_pending(&self) -> bool {
        let controller = Controller::new();
        controller.is_pending(Interrupt::Timer1)
    }
}
