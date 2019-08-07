//! Implement INode for Pipe

use alloc::{collections::vec_deque::VecDeque, string::String, sync::Arc};
use core::any::Any;

use rcore_fs::vfs::*;

use crate::sync::Condvar;
use crate::sync::SpinNoIrqLock as Mutex;

#[derive(Clone)]
pub enum PipeEnd {
    Read,
    Write,
}

pub struct PipeData {
    buf: VecDeque<u8>,
    new_data: Condvar,
}

#[derive(Clone)]
pub struct Pipe {
    data: Arc<Mutex<PipeData>>,
    direction: PipeEnd,
}

impl Pipe {
    /// Create a pair of INode: (read, write)
    pub fn create_pair() -> (Pipe, Pipe) {
        let inner = PipeData {
            buf: VecDeque::new(),
            new_data: Condvar::new(),
        };
        let data = Arc::new(Mutex::new(inner));
        (
            Pipe {
                data: data.clone(),
                direction: PipeEnd::Read,
            },
            Pipe {
                data: data.clone(),
                direction: PipeEnd::Write,
            },
        )
    }

    fn can_read(&self) -> bool {
        if let PipeEnd::Read = self.direction {
            self.data.lock().buf.len() > 0 || self.is_broken()
        } else {
            false
        }
    }

    fn can_write(&self) -> bool {
        if let PipeEnd::Write = self.direction {
            !self.is_broken()
        } else {
            false
        }
    }

    fn is_broken(&self) -> bool {
        Arc::strong_count(&self.data) < 2
    }
}

impl INode for Pipe {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        if let PipeEnd::Read = self.direction {
            let mut data = self.data.lock();
            if let Some(ch) = data.buf.pop_front() {
                buf[0] = ch;
                Ok(1)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        if let PipeEnd::Write = self.direction {
            if buf.len() > 0 {
                let mut data = self.data.lock();
                data.buf.push_back(buf[0]);
                data.new_data.notify_all();
                Ok(1)
            } else {
                Ok(0)
            }
        } else {
            Ok(0)
        }
    }

    fn poll(&self) -> Result<PollStatus> {
        Ok(PollStatus {
            read: self.can_read(),
            write: self.can_write(),
            error: false,
        })
    }

    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}
