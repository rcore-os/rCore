// Following copied from crate `x86_64`

use core::ops::{Index, IndexMut};

pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Idt {
        Idt {entries: [IdtEntry::new(); 256]}
    }
    pub fn load(&'static self) {
        use x86_64::instructions::tables::{DescriptorTablePointer, lidt};
        use core::mem::size_of;

        let ptr = DescriptorTablePointer {
            base: self as *const _ as u64,
            limit: (size_of::<Self>() - 1) as u16,
        };

        unsafe { lidt(&ptr) };
    }
}

impl Index<u8> for Idt {
    type Output = IdtEntry;
    fn index(&self, index: u8) -> &Self::Output {
        &self.entries[index as usize]
    }
}

impl IndexMut<u8> for Idt {
    fn index_mut(&mut self, index: u8) -> &mut Self::Output {
        &mut self.entries[index as usize]
    }
}

// Following copied from Redox

bitflags! {
    pub struct IdtFlags: u8 {
        const PRESENT = 1 << 7;
        const RING_0 = 0 << 5;
        const RING_1 = 1 << 5;
        const RING_2 = 2 << 5;
        const RING_3 = 3 << 5;
        const SS = 1 << 4;
        const INTERRUPT = 0xE;
        const TRAP = 0xF;
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct IdtEntry {
    offsetl: u16,
    selector: u16,
    zero: u8,
    attribute: u8,
    offsetm: u16,
    offseth: u32,
    zero2: u32
}

impl IdtEntry {
    pub const fn new() -> IdtEntry {
        IdtEntry {
            offsetl: 0,
            selector: 0,
            zero: 0,
            attribute: 0,
            offsetm: 0,
            offseth: 0,
            zero2: 0
        }
    }

    pub fn set_flags(&mut self, flags: IdtFlags) {
        self.attribute = flags.bits;
    }

    pub fn set_offset(&mut self, selector: u16, base: usize) {
        self.selector = selector;
        self.offsetl = base as u16;
        self.offsetm = (base >> 16) as u16;
        self.offseth = (base >> 32) as u32;
    }

    // A function to set the offset more easily
    pub fn set_handler_fn(&mut self, func: unsafe extern fn()) -> &mut Self {
        self.set_flags(IdtFlags::PRESENT | IdtFlags::RING_0 | IdtFlags::INTERRUPT);
        self.set_offset(8, func as usize);
        self
    }

    pub unsafe fn set_stack_index(&mut self, index: u16) -> &mut Self {
        // The hardware IST index starts at 1, but our software IST index
        // starts at 0. Therefore we need to add 1 here.
        self.offsetl &= !0b111;
        self.offsetl += index + 1;
        self
    }
}