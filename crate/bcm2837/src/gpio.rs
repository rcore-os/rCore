use crate::IO_BASE;
use crate::timer::delay;
use core::marker::PhantomData;
use volatile::{ReadOnly, Volatile, WriteOnly};

/// The base address of the `GPIO` registers.
const GPIO_BASE: usize = IO_BASE + 0x200000;

/// An alternative GPIO function. (ref: peripherals 6.1, page 92)
#[repr(u8)]
pub enum Function {
    Input = 0b000,
    Output = 0b001,
    Alt0 = 0b100,
    Alt1 = 0b101,
    Alt2 = 0b110,
    Alt3 = 0b111,
    Alt4 = 0b011,
    Alt5 = 0b010,
}

/// GPIO registers starting from `GPIO_BASE` (ref: peripherals 6.1, page 90)
#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    FSEL: [Volatile<u32>; 6],
    __reserved0: u32,
    SET: [WriteOnly<u32>; 2],
    __reserved1: u32,
    CLR: [WriteOnly<u32>; 2],
    __reserved2: u32,
    LEV: [ReadOnly<u32>; 2],
    __reserved3: u32,
    EDS: [Volatile<u32>; 2],
    __reserved4: u32,
    REN: [Volatile<u32>; 2],
    __reserved5: u32,
    FEN: [Volatile<u32>; 2],
    __reserved6: u32,
    HEN: [Volatile<u32>; 2],
    __reserved7: u32,
    LEN: [Volatile<u32>; 2],
    __reserved8: u32,
    AREN: [Volatile<u32>; 2],
    __reserved9: u32,
    AFEN: [Volatile<u32>; 2],
    __reserved10: u32,
    PUD: Volatile<u32>,
    PUDCLK: [Volatile<u32>; 2],
}

/// Possible states for a GPIO pin.
pub enum Uninitialized {}
pub enum Input {}
pub enum Output {}
pub enum Alt {}

/// A GPIO pin in state `State`.
///
/// The `State` generic always corresponds to an uninstantiatable type that is
/// use solely to mark and track the state of a given GPIO pin. A `Gpio`
/// structure starts in the `Uninitialized` state and must be transitions into
/// one of `Input`, `Output`, or `Alt` via the `into_input`, `into_output`, and
/// `into_alt` methods before it can be used.
pub struct Gpio<State> {
    pin: u8,
    registers: &'static mut Registers,
    _state: PhantomData<State>,
}

impl<T> Gpio<T> {
    /// Transitions `self` to state `S`, consuming `self` and returning a new
    /// `Gpio` instance in state `S`. This method should _never_ be exposed to
    /// the public!
    #[inline(always)]
    fn transition<S>(self) -> Gpio<S> {
        Gpio {
            pin: self.pin,
            registers: self.registers,
            _state: PhantomData,
        }
    }

    /// Set the Gpio pull-up/pull-down state for values in `pin_value`
    /// (ref: peripherals 6.1, page 101)
    pub fn set_gpio_pd(&mut self, pud_value: u8) {
        let index = if self.pin >= 32 { 1 } else { 0 };

        self.registers.PUD.write(pud_value as u32);
        delay(150);
        self.registers.PUDCLK[index as usize].write((1 << self.pin) as u32);
        delay(150);
        self.registers.PUD.write(0);
        self.registers.PUDCLK[index as usize].write(0);
    }
}

impl Gpio<Uninitialized> {
    /// Returns a new `GPIO` structure for pin number `pin`.
    ///
    /// # Panics
    ///
    /// Panics if `pin` > `53`.
    pub fn new(pin: u8) -> Gpio<Uninitialized> {
        if pin > 53 {
            panic!("Gpio::new(): pin {} exceeds maximum of 53", pin);
        }

        Gpio {
            registers: unsafe { &mut *(GPIO_BASE as *mut Registers) },
            pin: pin,
            _state: PhantomData,
        }
    }

    /// Enables the alternative function `function` for `self`. Consumes self
    /// and returns a `Gpio` structure in the `Alt` state.
    pub fn into_alt(self, function: Function) -> Gpio<Alt> {
        let select = (self.pin / 10) as usize;
        let offset = 3 * (self.pin % 10) as usize;
        self.registers.FSEL[select].update(|value| {
            *value &= !(0b111 << offset);
            *value |= (function as u32) << offset;
        });
        self.transition()
    }

    /// Sets this pin to be an _output_ pin. Consumes self and returns a `Gpio`
    /// structure in the `Output` state.
    pub fn into_output(self) -> Gpio<Output> {
        self.into_alt(Function::Output).transition()
    }

    /// Sets this pin to be an _input_ pin. Consumes self and returns a `Gpio`
    /// structure in the `Input` state.
    pub fn into_input(self) -> Gpio<Input> {
        self.into_alt(Function::Input).transition()
    }
}

impl Gpio<Output> {
    /// Sets (turns on) the pin.
    pub fn set(&mut self) {
        let index = if self.pin >= 32 { 1 } else { 0 };
        self.registers.SET[index as usize].write(1 << (self.pin - index * 32));
    }

    /// Clears (turns off) the pin.
    pub fn clear(&mut self) {
        let index = if self.pin >= 32 { 1 } else { 0 };
        self.registers.CLR[index as usize].write(1 << (self.pin - index * 32));
    }
}

impl Gpio<Input> {
    /// Reads the pin's value. Returns `true` if the level is high and `false`
    /// if the level is low.
    pub fn level(&mut self) -> bool {
        let index = if self.pin >= 32 { 1 } else { 0 };
        let high = 1 << (self.pin - index * 32);
        (self.registers.LEV[index as usize].read() & high) == high
    }
}
