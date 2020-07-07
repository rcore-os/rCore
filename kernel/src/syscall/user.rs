use super::SysError;
use crate::memory::{copy_from_user, copy_to_user};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;

#[repr(C)]
pub struct UserPtr<T, P: Policy> {
    ptr: *mut T,
    mark: PhantomData<P>,
}

pub trait Policy {}
pub trait Read: Policy {}
pub trait Write: Policy {}
pub enum In {}
pub enum Out {}
pub enum InOut {}

impl Policy for In {}
impl Policy for Out {}
impl Policy for InOut {}
impl Read for In {}
impl Write for Out {}
impl Read for InOut {}
impl Write for InOut {}

pub type UserInPtr<T> = UserPtr<T, In>;
pub type UserOutPtr<T> = UserPtr<T, Out>;
pub type UserInOutPtr<T> = UserPtr<T, InOut>;

type Result<T> = core::result::Result<T, SysError>;

impl<T, P: Policy> Debug for UserPtr<T, P> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.ptr)
    }
}

// TODO: this is a workaround for `clear_child_tid`.
unsafe impl<T, P: Policy> Send for UserPtr<T, P> {}
unsafe impl<T, P: Policy> Sync for UserPtr<T, P> {}

impl<T, P: Policy> From<usize> for UserPtr<T, P> {
    fn from(x: usize) -> Self {
        UserPtr {
            ptr: x as _,
            mark: PhantomData,
        }
    }
}

impl<T, P: Policy> UserPtr<T, P> {
    pub fn ptr(&self) -> *mut T {
        self.ptr
    }

    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }

    pub fn add(&self, count: usize) -> Self {
        UserPtr {
            ptr: unsafe { self.ptr.add(count) },
            mark: PhantomData,
        }
    }

    pub fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl<T, P: Read> UserPtr<T, P> {
    pub fn as_ref(&self) -> Result<&'static T> {
        Ok(unsafe { &*self.ptr })
    }

    pub fn read(&self) -> Result<T> {
        if let Some(res) = copy_from_user(self.ptr) {
            Ok(res)
        } else {
            Err(SysError::EFAULT)
        }
    }

    pub fn read_if_not_null(&self) -> Result<Option<T>> {
        if self.ptr.is_null() {
            return Ok(None);
        }
        let value = self.read()?;
        Ok(Some(value))
    }

    pub fn read_array(&self, len: usize) -> Result<Vec<T>> {
        if len == 0 {
            return Ok(Vec::default());
        }
        let mut ret = Vec::<T>::with_capacity(len);
        for i in 0..len {
            let ptr = unsafe { self.ptr.add(i) };
            if let Some(res) = copy_from_user(ptr) {
                ret.push(res)
            } else {
                return Err(SysError::EFAULT);
            }
        }
        Ok(ret)
    }
}

impl<P: Read> UserPtr<u8, P> {
    pub fn read_string(&self, len: usize) -> Result<String> {
        let src = self.read_array(len)?;
        let s = core::str::from_utf8(&src).map_err(|_| SysError::EINVAL)?;
        Ok(String::from(s))
    }

    pub fn read_cstring(&self) -> Result<String> {
        for i in 0.. {
            let ptr = unsafe { self.ptr.add(i) };
            if let Some(res) = copy_from_user(ptr) {
                if res == 0 {
                    // found
                    return self.read_string(i);
                }
            } else {
                return Err(SysError::EFAULT);
            }
        }
        Err(SysError::EINVAL)
    }
}

impl<T, P: Write> UserPtr<T, P> {
    pub fn write(&mut self, value: T) -> Result<()> {
        if copy_to_user(self.ptr, &value) {
            Ok(())
        } else {
            Err(SysError::EFAULT)
        }
    }

    pub fn write_if_not_null(&mut self, value: T) -> Result<()> {
        if self.ptr.is_null() {
            return Ok(());
        }
        self.write(value)
    }

    pub fn write_array(&mut self, values: &[T]) -> Result<()> {
        if values.is_empty() {
            return Ok(());
        }
        for i in 0..values.len() {
            let ptr = unsafe { self.ptr.add(i) };
            if !copy_to_user(ptr, &values[i]) {
                return Err(SysError::EFAULT);
            }
        }
        Ok(())
    }
}

impl<P: Write> UserPtr<u8, P> {
    pub fn write_cstring(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        self.write_array(bytes)?;
        let ptr = unsafe { self.ptr.add(bytes.len()) };
        let null = 0u8;
        if !copy_to_user(ptr, &null) {
            return Err(SysError::EFAULT);
        }
        Ok(())
    }
}
