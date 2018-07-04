extern crate riscv;
extern crate bbl;

pub fn test() {
    bbl::sbi::console_putchar(b'g' as u8 as u32);
}