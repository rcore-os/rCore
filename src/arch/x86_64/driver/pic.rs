// Copy from Redox

use redox_syscall::io::*;
use spin::Mutex;

static MASTER: Mutex<Pic> = Mutex::new(Pic::new(0x20));
static SLAVE: Mutex<Pic> = Mutex::new(Pic::new(0xA0));

pub fn disable() {
    // Mask all interrupts (Copy from xv6 x86_64)
    MASTER.lock().cmd.write(0xFF);
    SLAVE.lock().cmd.write(0xFF);
    debug!("pic: disabled");
}

pub fn init() {
    assert_has_not_been_called!("pic::init must be called only once");
    
    let mut master = MASTER.lock();
    let mut slave = SLAVE.lock();

    // Start initialization
    master.cmd.write(0x11);
    slave.cmd.write(0x11);

    // Set offsets
    master.data.write(0x20);
    slave.data.write(0x28);

    // Set up cascade
    master.data.write(4);
    slave.data.write(2);

    // Set up interrupt mode (1 is 8086/88 mode, 2 is auto EOI)
    master.data.write(1);
    slave.data.write(1);

    // Unmask interrupts
    master.data.write(0);
    slave.data.write(0);

    // Ack remaining interrupts
    master.ack();
    slave.ack();

    debug!("pic: init end");    
}

pub fn enable_irq(irq: u8)
{
    match irq {
        _ if irq < 8 => MASTER.lock().mask_set(irq),
        _ if irq < 16 => SLAVE.lock().mask_set(irq-8),
        _ => panic!("irq not in 0..16"),
    }
}

pub fn ack(irq: u8) {
    assert!(irq < 16);
    MASTER.lock().ack();
    if irq >= 8 {
        SLAVE.lock().ack();
    }
}

struct Pic {
    cmd: Pio<u8>,
    data: Pio<u8>,
}

impl Pic {
    const fn new(port: u16) -> Pic {
        Pic {
            cmd: Pio::new(port),
            data: Pio::new(port + 1),
        }
    }

    fn ack(&mut self) {
        self.cmd.write(0x20);
    }

    fn mask_set(&mut self, irq: u8) {
        assert!(irq < 8);

        let mut mask = self.data.read();
        mask |= 1 << irq;
        self.data.write(mask);
    }

    fn mask_clear(&mut self, irq: u8) {
        assert!(irq < 8);

        let mut mask = self.data.read();
        mask &= !(1 << irq);
        self.data.write(mask);
    }
}
