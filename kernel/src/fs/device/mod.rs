pub mod test;

#[cfg(not(feature = "link_user"))]
pub mod block;
pub mod disk;
#[cfg(feature = "link_user")]
pub mod membuf;

#[cfg(not(feature = "link_user"))]
pub use self::block::BlockDriver;
pub use self::disk::Disk;
#[cfg(feature = "link_user")]
pub use self::membuf::MemBuf;
