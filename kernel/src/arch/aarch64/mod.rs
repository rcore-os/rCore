//! Entrance and initialization for aarch64.

pub mod io;
pub mod paging;
pub mod memory;
pub mod interrupt;

#[cfg(feature = "board_raspi3")]
#[path = "board/raspi3/mod.rs"]
pub mod board;

pub use self::board::timer;

/// TODO
/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn rust_main() -> ! {
    // Init board to enable serial port.
    board::init();

    // First init log mod, so that we can print log info.
    // FIXME
    // ::logging::init();
    interrupt::init();
    timer::init();

    // ::process::init();

    unsafe { interrupt::enable(); }

    super::fs::show_logo();

    loop {
        print!(">> ");
        loop {
            let c = io::getchar();
            match c {
                '\u{7f}' => {
                    print!("\u{7f}");
                }
                'b' => unsafe {
                    println!("brk 233");
                    asm!("brk 233");
                },
                'c' => unsafe {
                    println!("sys_putc");
                    asm!(
                        "mov x8, #30
                         mov x0, #65
                         svc 0"
                    );
                },
                't' => unsafe {
                    println!("{}", timer::get_cycle());
                },
                ' '...'\u{7e}' => {
                    print!("{}", c);
                }
                '\n' | '\r' => {
                    print!("\n");
                    break;
                }
                _ => {}
            }
        }
    }

    // ::kmain();
}

global_asm!(include_str!("boot/boot.S"));
