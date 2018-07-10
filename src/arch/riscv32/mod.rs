extern crate riscv;
extern crate bbl;

pub mod serial;
pub mod interrupt;
pub mod timer;
pub mod paging;

pub fn init() {
    println!("Hello RISCV! {}", 123);
    interrupt::init();
//    timer::init();
    println!("satp: {:x?}", riscv::register::satp::read());

    use xmas_elf::ElfFile;
    use core::slice;
    use self::riscv::addr::*;
    let begin = 0x80400000usize;
    extern { fn end(); }
    let end = end as usize;
    println!("Kernel: {:#x} {:#x}", begin, end);
//    let kernel = unsafe{ slice::from_raw_parts(begin as *const u8, end - begin) };
//    let elf = ElfFile::new(kernel).unwrap();

    paging::setup_page_table(Frame::of_addr(PhysAddr::new(end as u32 + 4096)));

    loop {}
}