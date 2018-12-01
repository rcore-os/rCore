//! Entrance and initialization for aarch64.

pub mod io;
pub mod paging;
pub mod memory;
pub mod interrupt;
pub mod consts;
pub mod cpu;

#[cfg(feature = "board_raspi3")]
#[path = "board/raspi3/mod.rs"]
pub mod board;

pub use self::board::timer;

global_asm!(include_str!("boot/boot.S"));

/// The entry point of kernel
#[no_mangle] // don't mangle the name of this function
pub extern "C" fn rust_main() -> ! {
    // Enable mmu and paging
    memory::init_mmu_early();

    // Init board to enable serial port.
    board::init();

    crate::logging::init();
    interrupt::init();
    memory::init();
    timer::init();

    use crate::process::{processor, ContextImpl};
    crate::process::init();
    processor().manager().add(ContextImpl::new_kernel(kernel_proc2, 2333), 0);
    processor().manager().add(ContextImpl::new_user_test(kernel_proc3), 0);

    crate::kmain();
}

extern fn kernel_proc2(arg: usize) -> ! {
    use alloc::format;
    test_shell(&format!("proc2-{}>> ", arg));
}

extern fn kernel_proc3(arg: usize) -> ! {
    use alloc::format;
    test_shell(&format!("proc3-{}$ ", arg));
}

const LOGO: &str = r#"
    ____                __   ____  _____
   / __ \ __  __ _____ / /_ / __ \/ ___/
  / /_/ // / / // ___// __// / / /\__ \
 / _, _// /_/ /(__  )/ /_ / /_/ /___/ /
/_/ |_| \__,_//____/ \__/ \____//____/
"#;

pub fn show_logo() {
    println!("{}", LOGO);
}

#[inline(always)]
fn sys_call(id: usize, arg0: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize, arg5: usize) -> i32 {
    let ret: i32;
    unsafe {
        asm!("svc 0"
        : "={x0}" (ret)
        : "{x8}" (id), "{x0}" (arg0), "{x1}" (arg1), "{x2}" (arg2), "{x3}" (arg3), "{x4}" (arg4), "{x5}" (arg5)
        : "memory"
        : "volatile");
    }
    ret
}

pub fn test_shell(prefix: &str) -> ! {
    show_logo();
    loop {
        print!("{}", prefix);
        loop {
            let c = io::getchar();
            match c {
                '\u{7f}' => {
                    print!("\u{7f}");
                }
                'c' => unsafe {
                    print!("sys_putc: ");
                    sys_call(30, 'A' as usize, 0, 0, 0, 0, 0);
                },
                't' => unsafe {
                    println!("sys_get_time: {}", sys_call(17, 0, 0, 0, 0, 0, 0));
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
}
