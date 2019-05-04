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

    pub fn can_read(&self) -> bool {
        if let PipeEnd::Read = self.direction {
            self.data.lock().buf.len() > 0
        } else {
            false
        }
    }
}

// TODO: better way to provide default impl?
macro_rules! impl_inode {
    () => {
        fn poll(&self) -> Result<PollStatus> { Err(FsError::NotSupported) }
        fn metadata(&self) -> Result<Metadata> { Err(FsError::NotSupported) }
        fn set_metadata(&self, _metadata: &Metadata) -> Result<()> { Ok(()) }
        fn sync_all(&self) -> Result<()> { Ok(()) }
        fn sync_data(&self) -> Result<()> { Ok(()) }
        fn resize(&self, _len: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn create(&self, _name: &str, _type_: FileType, _mode: u32) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn unlink(&self, _name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn link(&self, _name: &str, _other: &Arc<INode>) -> Result<()> { Err(FsError::NotDir) }
        fn move_(&self, _old_name: &str, _target: &Arc<INode>, _new_name: &str) -> Result<()> { Err(FsError::NotDir) }
        fn find(&self, _name: &str) -> Result<Arc<INode>> { Err(FsError::NotDir) }
        fn get_entry(&self, _id: usize) -> Result<String> { Err(FsError::NotDir) }
        fn io_control(&self, _cmd: u32, _data: usize) -> Result<()> { Err(FsError::NotSupported) }
        fn fs(&self) -> Arc<FileSystem> { unimplemented!() }
        fn as_any_ref(&self) -> &Any { self }
    };
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
    impl_inode!();
}
