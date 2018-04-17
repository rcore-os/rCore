use syscall::io::{Io, Pio};

static mut PIT: Pit = Pit::new(0x40);

pub fn init() {
    assert_has_not_been_called!("pit::init must be called only once");
    unsafe{ PIT.init(100); }
    debug!("pit: init end");
}

struct Pit {
    chan0: Pio<u8>,
    chan1: Pio<u8>,
    chan2: Pio<u8>,
    command: Pio<u8>,
}

impl Pit {
    const fn new(port: u16) -> Self {
        Pit {
            chan0: Pio::new(port),
            chan1: Pio::new(port+1),
            chan2: Pio::new(port+2),
            command: Pio::new(port+3),
        }
    }
    pub fn init(&mut self, freq: u32) {
        self.command.write(TIMER_SEL0 | TIMER_RATEGEN | TIMER_16BIT);
        let div = Pit::divisor(freq);
        self.chan0.write((div & 0xFF) as u8);
        self.chan0.write((div >> 8) as u8);
    }
    fn divisor(freq: u32) -> u16 {
        let div = (TIMER_FREQ + freq / 2) / freq;
        assert!(div < 0x10000);
        div as u16
    }
}

const TIMER_FREQ    : u32 = 1193182;
const TIMER_SEL0    : u8 = 0x00;                    // select counter 0
const TIMER_RATEGEN : u8 = 0x04;                    // mode 2, rate generator
const TIMER_16BIT   : u8 = 0x30;                    // r/w counter 16 bits, LSB first