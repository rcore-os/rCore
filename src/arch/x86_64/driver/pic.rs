use syscall::io::*;

pub static mut MASTER: Pic = Pic::new(0x20);
pub static mut SLAVE: Pic = Pic::new(0xA0);

pub unsafe fn init() {
    // Start initialization
    MASTER.cmd.write(0x11);
    SLAVE.cmd.write(0x11);

    // Set offsets
    MASTER.data.write(0x20);
    SLAVE.data.write(0x28);

    // Set up cascade
    MASTER.data.write(4);
    SLAVE.data.write(2);

    // Set up interrupt mode (1 is 8086/88 mode, 2 is auto EOI)
    MASTER.data.write(1);
    SLAVE.data.write(1);

    // Unmask interrupts
    MASTER.data.write(0);
    SLAVE.data.write(0);

    // Ack remaining interrupts
    MASTER.ack();
    SLAVE.ack();
}

pub struct Pic {
    cmd: Pio<u8>,
    data: Pio<u8>,
}

impl Pic {
    pub const fn new(port: u16) -> Pic {
        Pic {
            cmd: Pio::new(port),
            data: Pio::new(port + 1),
        }
    }

    pub fn ack(&mut self) {
        self.cmd.write(0x20);
    }

    pub fn mask_set(&mut self, irq: u8) {
        assert!(irq < 8);

        let mut mask = self.data.read();
        mask |= 1 << irq;
        self.data.write(mask);
    }

    pub fn mask_clear(&mut self, irq: u8) {
        assert!(irq < 8);

        let mut mask = self.data.read();
        mask &= !(1 << irq);
        self.data.write(mask);
    }
}
