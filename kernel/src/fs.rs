use simple_filesystem::*;
use alloc::boxed::Box;
#[cfg(target_arch = "x86_64")]
use arch::driver::ide;
use spin::Mutex;

// Hard link user program
#[cfg(target_arch = "riscv32")]
global_asm!(r#"
    .section .rodata
    .align 12
_binary_user_riscv_img_start:
    .incbin "../user/user-riscv.img"
_binary_user_riscv_img_end:
"#);

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
        #[cfg(target_arch = "riscv32")]
            asm!("ecall"
            : "={x10}" (ret)
            : "{x10}" (id), "{x11}" (arg0), "{x12}" (arg1), "{x13}" (arg2), "{x14}" (arg3), "{x15}" (arg4), "{x16}" (arg5)
            : "memory"
            : "volatile");
        #[cfg(target_arch = "x86_64")]
            asm!("int 0x40"
            : "={rax}" (ret)
            : "{rax}" (id), "{rdi}" (arg0), "{rsi}" (arg1), "{rdx}" (arg2), "{rcx}" (arg3), "{r8}" (arg4), "{r9}" (arg5)
            : "memory"
            : "intel" "volatile");
        #[cfg(target_arch = "aarch64")]
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
            let c = super::arch::io::getchar();
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

pub fn shell() {
    show_logo();

    #[cfg(target_arch = "riscv32")]
    let device = {
        extern {
            fn _binary_user_riscv_img_start();
            fn _binary_user_riscv_img_end();
        }
        Box::new(unsafe { MemBuf::new(_binary_user_riscv_img_start, _binary_user_riscv_img_end) })
    };

    #[cfg(target_arch = "x86_64")]
    let device = Box::new(&ide::DISK1);

    #[cfg(target_arch = "aarch64")]
    // TODO
    let device: Box<dyn Device> = unimplemented!();

    let sfs = SimpleFileSystem::open(device).expect("failed to open SFS");
    let root = sfs.root_inode();
    let files = root.borrow().list().unwrap();
    println!("Available programs: {:?}", files);

    // Avoid stack overflow in release mode
    // Equal to: `buf = Box::new([0; 64 << 12])`
    use alloc::alloc::{alloc, dealloc, Layout};
    const BUF_SIZE: usize = 0x40000;
    let layout = Layout::from_size_align(BUF_SIZE, 0x1000).unwrap();
    let buf = unsafe{ slice::from_raw_parts_mut(alloc(layout), BUF_SIZE) };
    loop {
        print!(">> ");
        use console::get_line;
        let name = get_line();
        if name == "" {
            continue;
        }
        if let Ok(file) = root.borrow().lookup(name.as_str()) {
            use process::*;
            let len = file.borrow().read_at(0, &mut *buf).unwrap();
            let pid = processor().add(Context::new_user(&buf[..len]));
            processor().current_wait_for(pid);
        } else {
            println!("Program not exist");
        }
    }
    unsafe { dealloc(buf.as_mut_ptr(), layout) };
}

struct MemBuf(&'static [u8]);

impl MemBuf {
    unsafe fn new(begin: unsafe extern fn(), end: unsafe extern fn()) -> Self {
        use core::slice;
        MemBuf(slice::from_raw_parts(begin as *const u8, end as usize - begin as usize))
    }
}

impl Device for MemBuf {
    fn read_at(&mut self, offset: usize, buf: &mut [u8]) -> Option<usize> {
        let slice = self.0;
        let len = buf.len().min(slice.len() - offset);
        buf[..len].copy_from_slice(&slice[offset..offset + len]);
        Some(len)
    }
    fn write_at(&mut self, offset: usize, buf: &[u8]) -> Option<usize> {
        None
    }
}

use core::slice;

#[cfg(target_arch = "x86_64")]
impl BlockedDevice for &'static ide::DISK1 {
    const BLOCK_SIZE_LOG2: u8 = 9;
    fn read_at(&mut self, block_id: usize, buf: &mut [u8]) -> bool {
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts_mut(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.0.lock().read(block_id as u64, 1, buf).is_ok()
    }
    fn write_at(&mut self, block_id: usize, buf: &[u8]) -> bool {
        assert!(buf.len() >= ide::BLOCK_SIZE);
        let buf = unsafe { slice::from_raw_parts(buf.as_ptr() as *mut u32, ide::BLOCK_SIZE / 4) };
        self.0.lock().write(block_id as u64, 1, buf).is_ok()
    }
}
