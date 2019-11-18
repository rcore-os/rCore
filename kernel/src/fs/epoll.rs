use crate::fs::FileLike;
use crate::memory::MemorySet;
use crate::process::Process;
use crate::sync::{Condvar, SpinNoIrqLock};
use crate::syscall::{SysError, SysResult};
use alloc::{collections::BTreeMap, collections::BTreeSet};
use core::mem::size_of;
use core::slice;

pub struct EpollInstance {
    pub events: BTreeMap<usize, EpollEvent>,
    pub readyList: SpinNoIrqLock<BTreeSet<usize>>,
    pub newCtlList: SpinNoIrqLock<BTreeSet<usize>>,
}

impl Clone for EpollInstance {
    fn clone(&self) -> Self {
        EpollInstance::new(0)
    }
}

impl EpollInstance {
    pub fn new(flags: usize) -> Self {
        return EpollInstance {
            events: BTreeMap::new(),
            readyList: Default::default(),
            newCtlList: Default::default(),
        };
    }

    pub fn control(&mut self, op: usize, fd: usize, event: &EpollEvent) -> SysResult {
        match (op as i32) {
            EPollCtlOp::ADD => {
                self.events.insert(fd, event.clone());
                self.newCtlList.lock().insert(fd);
            }

            EPollCtlOp::MOD => {
                if self.events.get(&fd).is_some() {
                    self.events.remove(&fd);
                    self.events.insert(fd, event.clone());
                    self.newCtlList.lock().insert(fd);
                } else {
                    return Err(SysError::EPERM);
                }
            }

            EPollCtlOp::DEL => {
                if self.events.get(&fd).is_some() {
                    self.events.remove(&fd);
                } else {
                    return Err(SysError::EPERM);
                }
            }
            _ => {
                return Err(SysError::EPERM);
            }
        }
        Ok(0)
    }
}

#[derive(Clone, Copy)]
pub struct EpollData {
    ptr: u64,
}

#[repr(packed)]
#[derive(Clone)]
pub struct EpollEvent {
    pub events: u32,     /* Epoll events */
    pub data: EpollData, /* User data variable */
}

impl EpollEvent {
    pub const EPOLLIN: u32 = 0x001;
    pub const EPOLLOUT: u32 = 0x004;
    pub const EPOLLERR: u32 = 0x008;
    pub const EPOLLHUP: u32 = 0x010;

    pub const EPOLLPRI: u32 = 0x002;
    pub const EPOLLRDNORM: u32 = 0x040;
    pub const EPOLLRDBAND: u32 = 0x080;
    pub const EPOLLWRNORM: u32 = 0x100;
    pub const EPOLLWRBAND: u32 = 0x200;
    pub const EPOLLMSG: u32 = 0x400;
    pub const EPOLLRDHUP: u32 = 0x2000;
    pub const EPOLLEXCLUSIVE: u32 = 1 << 28;
    pub const EPOLLWAKEUP: u32 = 1 << 29;
    pub const EPOLLONESHOT: u32 = 1 << 30;
    pub const EPOLLET: u32 = 1 << 31;

    pub fn contains(&self, events: u32) -> bool {
        if (self.events & events) == 0 {
            return false;
        } else {
            return true;
        }
    }
}

pub struct EPollCtlOp;
impl EPollCtlOp {
    const ADD: i32 = 1; /* Add a file descriptor to the interface.  */
    const DEL: i32 = 2; /* Remove a file descriptor from the interface.  */
    const MOD: i32 = 3; /* Change file descriptor epoll_event structure.  */
}

impl Process {
    pub fn get_epoll_instance_mut(&mut self, fd: usize) -> Result<&mut EpollInstance, SysError> {
        match self.get_file_like(fd)? {
            FileLike::EpollInstance(instance) => Ok(instance),
            _ => Err(SysError::EPERM),
        }
    }

    pub fn get_epoll_instance(&self, fd: usize) -> Result<&EpollInstance, SysError> {
        match self.files.get(&fd) {
            Some(file_like) => match file_like {
                FileLike::EpollInstance(instance) => Ok(&instance),
                _ => Err(SysError::EPERM),
            },
            None => {
                return Err(SysError::EPERM);
            }
        }
    }
}
