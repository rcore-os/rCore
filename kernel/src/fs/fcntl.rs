// currently support x86_64 only
// copy from fcntl.h

pub const F_DUPFD: usize = 0; /* dup */
pub const F_GETFD: usize = 1; /* get close_on_exec */
pub const F_SETFD: usize = 2; /* set/clear close_on_exec */
pub const F_GETFL: usize = 3; /* get file->f_flags */
pub const F_SETFL: usize = 4; /* set file->f_flags */
pub const F_GETLK: usize = 5; /* Get record locking info.  */
pub const F_SETLK: usize = 6; /* Set record locking info (non-blocking).  */
pub const F_SETLKW: usize = 7; /* Set record locking info (blocking).  */

const F_LINUX_SPECIFIC_BASE: usize = 1024;

pub const FD_CLOEXEC: usize = 1;
pub const F_DUPFD_CLOEXEC: usize = F_LINUX_SPECIFIC_BASE + 6;

pub const O_NONBLOCK: usize = 0o4000;
pub const O_APPEND: usize = 0o2000;
pub const O_CLOEXEC: usize = 0o2000000; /* set close_on_exec */

pub const AT_SYMLINK_NOFOLLOW: usize = 0x100;
