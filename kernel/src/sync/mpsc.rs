use super::Condvar;
use super::SpinLock as Mutex;
use alloc::{collections::VecDeque, sync::Arc, sync::Weak};

struct Channel<T> {
    deque: Mutex<VecDeque<T>>,
    pushed: Condvar,
}

impl<T> Default for Channel<T> {
    fn default() -> Self {
        Channel {
            deque: Mutex::<_>::default(),
            pushed: Condvar::default(),
        }
    }
}

/// The receiving half of Rust's channel (or sync_channel) type.
/// This half can only be owned by one thread.
///
/// Messages sent to the channel can be retrieved using recv.
pub struct Receiver<T> {
    inner: Arc<Channel<T>>,
}

unsafe impl<T: Send> Send for Receiver<T> {}

impl<T> !Sync for Receiver<T> {}

#[derive(Debug)]
pub struct RecvError;

impl<T> Receiver<T> {
    /// Attempts to wait for a value on this receiver,
    /// returning an error if the corresponding channel has hung up.
    pub fn recv(&self) -> Result<T, RecvError> {
        let mut deque = self.inner.deque.lock();
        while deque.is_empty() {
            deque = self.inner.pushed.wait(deque);
        }
        Ok(deque.pop_front().unwrap())
    }
}

/// The sending-half of Rust's asynchronous channel type.
/// This half can only be owned by one thread, but it can be cloned to send to other threads.
///
/// Messages can be sent through this channel with send.
#[derive(Clone)]
pub struct Sender<T> {
    inner: Weak<Channel<T>>,
}

unsafe impl<T: Send> Send for Sender<T> {}

impl<T> !Sync for Sender<T> {}

#[derive(Debug)]
pub struct SendError<T>(pub T);

impl<T> Sender<T> {
    /// Attempts to send a value on this channel,
    /// returning it back if it could not be sent.
    pub fn send(&self, t: T) -> Result<(), SendError<T>> {
        match self.inner.upgrade() {
            None => Err(SendError(t)),
            Some(inner) => {
                let mut deque = inner.deque.lock();
                deque.push_back(t);
                inner.pushed.notify_one();
                Ok(())
            }
        }
    }
}

/// Creates a new asynchronous channel, returning the sender/receiver halves.
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let channel = Arc::new(Channel::<T>::default());
    let sender = Sender {
        inner: Arc::downgrade(&channel),
    };
    let receiver = Receiver { inner: channel };
    (sender, receiver)
}

pub mod test {
    //! Copied from std::mpsc::test

    use super::*;
    use crate::thread;
    use alloc::boxed::Box;

    fn smoke() {
        let (tx, rx) = channel::<i32>();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
    }

    fn drop_full() {
        let (tx, _rx) = channel::<Box<isize>>();
        tx.send(Box::new(1)).unwrap();
    }

    fn drop_full_shared() {
        let (tx, _rx) = channel::<Box<isize>>();
        drop(tx.clone());
        drop(tx.clone());
        tx.send(Box::new(1)).unwrap();
    }

    fn smoke_shared() {
        let (tx, rx) = channel::<i32>();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
        let tx = tx.clone();
        tx.send(1).unwrap();
        assert_eq!(rx.recv().unwrap(), 1);
    }

    fn smoke_threads() {
        let (tx, rx) = channel::<i32>();
        let _t = thread::spawn(move || {
            tx.send(1).unwrap();
        });
        assert_eq!(rx.recv().unwrap(), 1);
    }

    fn smoke_port_gone() {
        let (tx, rx) = channel::<i32>();
        drop(rx);
        assert!(tx.send(1).is_err());
    }

    pub fn test_all() {
        smoke();
        drop_full();
        drop_full_shared();
        smoke_shared();
        smoke_threads();
        smoke_port_gone();
        println!("mpsc test end");
    }
}
