use crate::consts::IO_BASE;
use volatile::{ReadOnly, Volatile, WriteOnly};

/// The base address for the `MU` registers.
const MAILBOX_BASE: usize = IO_BASE + 0xB000 + 0x880;

/// Available mailbox channels
///
/// (ref: https://github.com/raspberrypi/firmware/wiki/Mailboxes)
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum MailboxChannel {
    Framebuffer = 1,
    Property = 8,
}

/// Read from mailbox status register (MAILx_STA).
#[repr(u32)]
enum MailboxStatus {
    MailboxEmpty = 1 << 30,
    MailboxFull = 1 << 31,
}

/// Mailbox registers. We basically only support mailbox 0 & 1. We
/// deliver to the VC in mailbox 1, it delivers to us in mailbox 0. See
/// BCM2835-ARM-Peripherals.pdf section 1.3 for an explanation about
/// the placement of memory barriers.
///
/// (ref: https://github.com/raspberrypi/firmware/wiki/Mailboxes)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    MAIL0_RD: ReadOnly<u32>, // 0x00
    __reserved0: [u32; 3],
    MAIL0_POL: ReadOnly<u32>, // 0x10
    MAIL0_SND: ReadOnly<u32>, // 0x14
    MAIL0_STA: ReadOnly<u32>, // 0x18
    MAIL0_CNF: Volatile<u32>, // 0x1c

    MAIL1_WRT: WriteOnly<u32>, // 0x20
    __reserved1: [u32; 3],
    _MAIL1_POL: ReadOnly<u32>, // 0x30
    _MAIL1_SND: ReadOnly<u32>, // 0x34
    MAIL1_STA: ReadOnly<u32>,  // 0x38
    _MAIL1_CNF: Volatile<u32>, // 0x3c
}

/// The Raspberry Pi's mailbox.
///
/// (ref: https://github.com/raspberrypi/firmware/wiki/Accessing-mailboxes)
pub struct Mailbox {
    registers: &'static mut Registers,
}

impl Mailbox {
    /// Returns a new instance of `Mailbox`.
    #[inline]
    pub fn new() -> Mailbox {
        Mailbox {
            registers: unsafe { &mut *(MAILBOX_BASE as *mut Registers) },
        }
    }

    /// Read from the requested channel of mailbox 0.
    pub fn read(&self, channel: MailboxChannel) -> u32 {
        loop {
            while self.registers.MAIL0_STA.read() & (MailboxStatus::MailboxEmpty as u32) != 0 {}
            let data = self.registers.MAIL0_RD.read();
            if data & 0xF == channel as u32 {
                return data & !0xF;
            }
        }
    }

    /// Write to the requested channel of mailbox 1.
    pub fn write(&mut self, channel: MailboxChannel, data: u32) {
        while self.registers.MAIL1_STA.read() & (MailboxStatus::MailboxFull as u32) != 0 {}
        self.registers.MAIL1_WRT.write((data & !0xF) | (channel as u32));
    }
}
