//! Implement INode for Stdin & Stdout

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::any::Any;

use rcore_fs::vfs::*;

use super::tty::TTY;
use crate::fs::devfs::foreground_pgid;
use crate::fs::ioctl::*;
use crate::process::process_group;
use crate::signal::{send_signal, Siginfo, Signal, SI_KERNEL};
use crate::sync::SpinNoIrqLock as Mutex;
use crate::sync::{Condvar, Event, EventBus};
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use spin::RwLock;

#[derive(Default)]
pub struct Stdin {
    buf: Mutex<VecDeque<u8>>,
    eventbus: Mutex<EventBus>,
    winsize: RwLock<Winsize>,
    termios: RwLock<Termois>,
}

impl Stdin {
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

#[derive(Default)]
pub struct Stdout {
    winsize: RwLock<Winsize>,
}

lazy_static! {
    pub static ref STDIN: Arc<Stdin> = Arc::new(Stdin::default());
    pub static ref STDOUT: Arc<Stdout> = Arc::new(Stdout::default());
}

impl INode for Stdin {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        if self.can_read() {
            buf[0] = self.pop() as u8;
            Ok(1)
        } else {
            Err(FsError::Again)
        }
    }
    fn write_at(&self, _offset: usize, _buf: &[u8]) -> Result<usize> {
        unimplemented!()
    }
    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: self.can_read(),
            write: false,
            error: false,
        })
    }

    fn async_poll<'a>(
        &'a self,
    ) -> Pin<Box<dyn Future<Output = Result<PollStatus>> + Send + Sync + 'a>> {
        struct SerialFuture<'a> {
            stdin: &'a Stdin,
        };

        impl<'a> Future for SerialFuture<'a> {
            type Output = Result<PollStatus>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
                if self.stdin.can_read() {
                    return Poll::Ready(self.stdin.poll());
                }
                let waker = cx.waker().clone();
                self.stdin.eventbus.lock().subscribe(Box::new({
                    move |_| {
                        waker.wake_by_ref();
                        true
                    }
                }));
                Poll::Pending
            }
        }

        Box::pin(SerialFuture { stdin: self })
    }

    fn io_control(&self, cmd: u32, data: usize) -> Result<usize> {
        match cmd as usize {
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
            TIOCGPGRP => {
                // pretend to be have a tty process group
                // Get the process group ID of the foreground process group on
                // this terminal.
                // TODO: verify pointer
                unsafe { *(data as *mut u32) = 0 };
                Ok(0)
            }
            TIOCSPGRP => {
                let gid = unsafe { *(data as *const i32) };
                info!("set foreground process group id to {}", gid);
                Ok(0)
            }
            _ => Err(FsError::NotSupported),
        }
    }
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

impl INode for Stdout {
    fn read_at(&self, _offset: usize, _buf: &mut [u8]) -> Result<usize> {
        unimplemented!()
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        use core::str;
        // we do not care the utf-8 things, we just want to print it!
        let s = unsafe { str::from_utf8_unchecked(buf) };
        print!("{}", s);
        Ok(buf.len())
    }

    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: false,
            write: true,
            error: false,
        })
    }

    fn io_control(&self, cmd: u32, data: usize) -> Result<usize> {
        match cmd as usize {
            TIOCGWINSZ => {
                let winsize = data as *mut Winsize;
                unsafe {
                    *winsize = *self.winsize.read();
                }
                Ok(0)
            }
            TCSETS | TCGETS | TIOCSPGRP => {
                // pretend to be tty
                Ok(0)
            }
            TIOCGPGRP => {
                // pretend to be have a tty process group
                // TODO: verify pointer
                unsafe { *(data as *mut u32) = 0 };
                Ok(0)
            }
            _ => Err(FsError::NotSupported),
        }
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
