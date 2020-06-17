use crate::fs::ioctl::*;
use crate::process::{process_group, Pgid};
use crate::signal::{send_signal, Signal};
use crate::signal::{Siginfo, SI_KERNEL};
use crate::{sync::Event, sync::EventBus, syscall::SysError};
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::any::Any;
use core::future::Future;
use core::pin::Pin;
use core::task::Context;
use core::task::Poll;
use rcore_fs::vfs::FsError::NotSupported;
use rcore_fs::vfs::*;
use spin::{Mutex, RwLock};

/// console tty
// Ref: [https://linux.die.net/man/4/tty]
#[derive(Default)]
pub struct TtyINode {
    /// foreground process group
    foreground_pgid: RwLock<Pgid>,
    buf: Mutex<VecDeque<u8>>,
    eventbus: Mutex<EventBus>,
    winsize: RwLock<Winsize>,
    termios: RwLock<Termois>,
}

lazy_static! {
    pub static ref TTY: Arc<TtyINode> = Arc::new(TtyINode::default());
}

pub fn foreground_pgid() -> Pgid {
    *TTY.foreground_pgid.read()
}

impl TtyINode {
    pub fn push(&self, c: u8) {
        let lflag = LocalModes::from_bits_truncate(self.termios.read().lflag);
        if lflag.contains(LocalModes::ISIG) && [0o3, 0o34, 0o32, 0o31].contains(&(c as i32)) {
            use Signal::*;
            let foregroud_processes = process_group(foreground_pgid());
            match c as i32 {
                // INTR
                0o3 => {
                    for proc in foregroud_processes {
                        send_signal(
                            proc,
                            -1,
                            Siginfo {
                                signo: SIGINT as i32,
                                errno: 0,
                                code: SI_KERNEL,
                                field: Default::default(),
                            },
                        );
                    }
                }
                _ => warn!("special char {} is unimplented", c),
            }
        } else {
            self.buf.lock().push_back(c);
            self.eventbus.lock().set(Event::READABLE);
        }
    }

    pub fn pop(&self) -> u8 {
        let mut buf_lock = self.buf.lock();
        let c = buf_lock.pop_front().unwrap();
        if buf_lock.len() == 0 {
            self.eventbus.lock().clear(Event::READABLE);
        }
        return c;
    }

    pub fn can_read(&self) -> bool {
        return self.buf.lock().len() > 0;
    }
}

impl INode for TtyINode {
    /// Read bytes at `offset` into `buf`, return the number of bytes read.
    fn read_at(&self, offset: usize, buf: &mut [u8]) -> Result<usize> {
        if self.can_read() {
            buf[0] = self.pop() as u8;
            Ok(1)
        } else {
            Err(FsError::Again)
        }
    }

    /// Write bytes at `offset` from `buf`, return the number of bytes written.
    fn write_at(&self, offset: usize, buf: &[u8]) -> Result<usize> {
        use core::str;
        // we do not care the utf-8 things, we just want to print it!
        let s = unsafe { str::from_utf8_unchecked(buf) };
        print!("{}", s);
        Ok(buf.len())
    }

    /// Poll the events, return a bitmap of events.
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: self.can_read(),
            write: true,
            error: false,
        })
    }

    fn async_poll<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<PollStatus>> + Send + Sync + 'a>> {
        struct SerialFuture<'a> {
            tty: &'a TtyINode,
        };

        impl<'a> Future for SerialFuture<'a> {
            type Output = Result<PollStatus>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                if self.tty.can_read() {
                    return Poll::Ready(self.tty.poll());
                }
                let waker = cx.waker().clone();
                self.tty.eventbus.lock().subscribe(Box::new({
                    move |_| {
                        waker.wake_by_ref();
                        true
                    }
                }));
                Poll::Pending
            }
        }

        Box::pin(SerialFuture { tty: self })
    }

    fn io_control(&self, cmd: u32, data: usize) -> Result<usize> {
        let cmd = cmd as usize;
        match cmd {
            TIOCGPGRP => {
                // TODO: check the pointer?
                let argp = data as *mut i32; // pid_t
                unsafe { *argp = *self.foreground_pgid.read() };
                Ok(0)
            }
            TIOCSPGRP => {
                let fpgid = unsafe { *(data as *const i32) };
                *self.foreground_pgid.write() = fpgid;
                info!("tty: set foreground process group to {}", fpgid);
                Ok(0)
            }
            TIOCGWINSZ => {
                let winsize = data as *mut Winsize;
                unsafe {
                    *winsize = *self.winsize.read();
                }
                Ok(0)
            }
            TCGETS => {
                let termois = data as *mut Termois;
                unsafe {
                    *termois = *self.termios.read();
                }
                let lflag = LocalModes::from_bits_truncate(self.termios.read().lflag);
                info!("get lfags: {:?}", lflag);
                Ok(0)
            }
            TCSETS => {
                let termois = data as *const Termois;
                unsafe {
                    *self.termios.write() = *termois;
                }
                let lflag = LocalModes::from_bits_truncate(self.termios.read().lflag);
                info!("set lfags: {:?}", lflag);
                Ok(0)
            }
            _ => Err(NotSupported),
        }
    }

    /// Get metadata of the INode
    fn metadata(&self) -> Result<Metadata> {
        Ok(Metadata {
            dev: 1,
            inode: 13,
            size: 0,
            blk_size: 0,
            blocks: 0,
            atime: Timespec { sec: 0, nsec: 0 },
            mtime: Timespec { sec: 0, nsec: 0 },
            ctime: Timespec { sec: 0, nsec: 0 },
            type_: FileType::CharDevice,
            mode: 0o666,
            nlinks: 1,
            uid: 0,
            gid: 0,
            rdev: make_rdev(5, 0),
        })
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
