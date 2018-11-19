extern crate cortex_a;

use self::cortex_a::regs::*;
use volatile::*;

/// The base address for the ARM generic timer, IRQs, mailboxes
const GEN_TIMER_REG_BASE: usize = 0x40000000;

/// Core interrupt sources (ref: QA7 4.10, page 16)
#[repr(u8)]
#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Copy, Clone, PartialEq, Debug)]
enum CoreInterrupt {
    CNTPSIRQ = 0,
    CNTPNSIRQ = 1,
    CNTHPIRQ = 2,
    CNTVIRQ = 3,
    Mailbox0 = 4,
    Mailbox1 = 5,
    Mailbox2 = 6,
    Mailbox3 = 7,
    Gpu = 8,
    Pmu = 9,
    AxiOutstanding = 10,
    LocalTimer = 11,
}

/// Timer, IRQs, mailboxes registers (ref: QA7 chapter 4, page 7)
#[allow(non_snake_case)]
#[repr(C)]
struct Registers {
    CONTROL: Volatile<u32>,
    _unused1: [Volatile<u32>; 8],
    LOCAL_IRQ: Volatile<u32>,
    _unused2: [Volatile<u32>; 3],
    LOCAL_TIMER_CTL: Volatile<u32>,
    LOCAL_TIMER_FLAGS: Volatile<u32>,
    _unused3: Volatile<u32>,
    CORE_TIMER_IRQCNTL: [Volatile<u32>; 4],
    CORE_MAILBOX_IRQCNTL: [Volatile<u32>; 4],
    CORE_IRQ_SRC: [Volatile<u32>; 4],
}

/// The ARM generic timer.
pub struct Timer {
    registers: &'static mut Registers,
}

impl Timer {
    /// Returns a new instance of `Timer`.
    pub fn new() -> Timer {
        Timer {
            registers: unsafe { &mut *(GEN_TIMER_REG_BASE as *mut Registers) },
        }
    }

    /// Reads the generic timer's counter and returns the 64-bit counter value.
    /// The returned value is the number of elapsed microseconds.
    pub fn read(&self) -> u64 {
        let cntfrq = CNTFRQ_EL0.get();
        (CNTPCT_EL0.get() * 1000000 / (cntfrq as u64)) as u64
    }

    /// Sets up a match in timer 1 to occur `us` microseconds from now. If
    /// interrupts for timer 1 are enabled and IRQs are unmasked, then a timer
    /// interrupt will be issued in `us` microseconds.
    pub fn tick_in(&mut self, us: u32) {
        let cntfrq = CNTFRQ_EL0.get();
        CNTP_TVAL_EL0.set(((cntfrq as f64) * (us as f64) / 1000000.0) as u32);
    }

    /// Initialization timer
    pub fn init(&mut self) {
        self.registers.CORE_TIMER_IRQCNTL[0].write(1 << (CoreInterrupt::CNTPNSIRQ as u8));
        CNTP_CTL_EL0.write(CNTP_CTL_EL0::ENABLE::SET);
    }

    /// Returns `true` if timer interruption is pending. Otherwise, returns `false`.
    pub fn is_pending(&self) -> bool {
        self.registers.CORE_IRQ_SRC[0].read() & (1 << (CoreInterrupt::CNTPNSIRQ as u8)) != 0
    }
}
