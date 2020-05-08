//! Implement INode for Pipe

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use core::any::Any;

use rcore_fs::vfs::*;

use crate::sync::Condvar;
use crate::sync::SpinNoIrqLock as Mutex;
use alloc::boxed::Box;
use core::cmp::min;
use rcore_fs::vfs::FsError::Again;
use rcore_thread::std_thread::{park, yield_now};

#[derive(Clone)]
pub enum PipeEnd {
    Read,
    Write,
}

pub struct PipeData {
    buf: VecDeque<u8>,
    new_data: Arc<Condvar>,
    end_cnt: i32,
}

#[derive(Clone)]
pub struct Pipe {
    data: Arc<Mutex<PipeData>>,
    direction: PipeEnd,
}

impl Drop for Pipe {
    fn drop(&mut self) {
        let mut data = self.data.lock();
        data.end_cnt -= 1;
        data.new_data.notify_all();
    }
}

impl Pipe {
    /// Create a pair of INode: (read, write)
    pub fn create_pair() -> (Pipe, Pipe) {
        let inner = PipeData {
            buf: VecDeque::new(),
            new_data: Arc::new(Condvar::new()),
            end_cnt: 2,
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
            // true
            let data = self.data.lock();
            data.buf.len() > 0 || data.end_cnt < 2
        } else {
            false
        }
    }

    fn can_write(&self) -> bool {
        if let PipeEnd::Write = self.direction {
            self.data.lock().end_cnt == 2
        } else {
            false
        }
    }

    // deprecate because of deadlock
    // fn is_broken(&self) -> bool {
    //     self.data.lock().end_cnt < 2
    // }
}

impl INode for Pipe {
    fn read_at(&self, _offset: usize, buf: &mut [u8]) -> Result<usize> {
        if buf.len() == 0 {
            return Ok(0);
        }
        if let PipeEnd::Read = self.direction {
            // condvar is fake
            // TODO: release on process lock?
            let mut data = self.data.lock();
            while data.buf.len() == 0 && data.end_cnt == 2 {
                data = data.new_data.clone().wait(data);
            }
            let len = min(buf.len(), data.buf.len());
            for i in 0..len {
                buf[i] = data.buf.pop_front().unwrap();
            }
            Ok(len)
        } else {
            Ok(0)
        }
    }

    fn write_at(&self, _offset: usize, buf: &[u8]) -> Result<usize> {
        if let PipeEnd::Write = self.direction {
            let mut data = self.data.lock();
            // data.buf.push_back(buf[0]);
            for c in buf {
                data.buf.push_back(*c);
            }
            data.new_data.notify_all();
            Ok(buf.len())
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
