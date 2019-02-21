use crate::syscall::{sys_close, sys_dup2, sys_exit, sys_open};
use crate::io::{O_RDONLY, O_WRONLY, STDIN, STDOUT};
use crate::ALLOCATOR;

use core::alloc::Layout;
use core::panic::PanicInfo;

#[linkage = "weak"]
#[no_mangle]
fn main() {
    panic!("No main() linked");
}

fn initfd(fd2: usize, path: &str, open_flags: usize) -> i32 {
    let fd1 = sys_open(path, open_flags);
    if fd1 < 0 {
        return fd1;
    }
    let mut ret = fd1;
    let fd1 = fd1 as usize;
    if fd1 != fd2 {
        sys_close(fd2);
        ret = sys_dup2(fd1, fd2);
        sys_close(fd1);
    }
    return ret;
}

fn init_heap() {
    const HEAP_SIZE: usize = 0x1000;
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe { ALLOCATOR.lock().init(HEAP.as_ptr() as usize, HEAP_SIZE); }
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> ! {
    let fd = initfd(STDIN, "stdin:", O_RDONLY);
    if fd < 0 {
        panic!("open <stdin> failed: {}.", fd);
    }
    let fd = initfd(STDOUT, "stdout:", O_WRONLY);
    if fd < 0 {
        panic!("open <stdout> failed: {}.", fd);
    }

    init_heap();
    main();
    sys_exit(0)
}

#[lang = "eh_personality"]
fn eh_personality() {}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let location = info.location().unwrap();
    let message = info.message().unwrap();
    println!("\n\nPANIC in {} at line {}\n    {}", location.file(), location.line(), message);
    sys_exit(1)
}

#[lang = "oom"]
fn oom(_: Layout) -> ! {
    panic!("out of memory");
}

#[no_mangle]
pub extern "C" fn abort() -> ! {
    sys_exit(2)
}
