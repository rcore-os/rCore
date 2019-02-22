//! System call

use alloc::{string::String, sync::Arc, vec::Vec};
use core::{slice, str};

use bitflags::bitflags;
use log::*;
use simple_filesystem::{FileInfo, FileType, FsError, INode};
use spin::{Mutex, MutexGuard};

use crate::arch::interrupt::TrapFrame;
use crate::fs::File;
use crate::process::*;
use crate::thread;
use crate::util;

use self::fs::*;
use self::proc::*;
use self::time::*;

mod fs;
mod proc;
mod time;

/// System call dispatcher
pub fn syscall(id: usize, args: [usize; 6], tf: &mut TrapFrame) -> isize {
    let ret = match id {
        // file
        000 => sys_read(args[0], args[1] as *mut u8, args[2]),
        001 => sys_write(args[0], args[1] as *const u8, args[2]),
        002 => sys_open(args[0] as *const u8, args[1]),
        003 => sys_close(args[0]),
//        004 => sys_stat(),
        005 => sys_fstat(args[0], args[1] as *mut Stat),
//        007 => sys_poll(),
//        008 => sys_lseek(),
//        009 => sys_mmap(),
//        011 => sys_munmap(),
//        013 => sys_sigaction(),
//        019 => sys_readv(),
//        020 => sys_writev(),
//        021 => sys_access(),
        024 => sys_yield(),
        033 => sys_dup2(args[0], args[1]),
//        034 => sys_pause(),
        035 => sys_sleep(args[0]), // TODO: nanosleep
        039 => sys_getpid(),
//        040 => sys_getppid(),
//        041 => sys_socket(),
//        042 => sys_connect(),
//        043 => sys_accept(),
//        044 => sys_sendto(),
//        045 => sys_recvfrom(),
//        046 => sys_sendmsg(),
//        047 => sys_recvmsg(),
//        048 => sys_shutdown(),
//        049 => sys_bind(),
//        050 => sys_listen(),
//        054 => sys_setsockopt(),
//        055 => sys_getsockopt(),
//        056 => sys_clone(),
        057 => sys_fork(tf),
        059 => sys_exec(args[0] as *const u8, args[1] as usize, args[2] as *const *const u8, tf),
        060 => sys_exit(args[0] as isize),
        061 => sys_wait(args[0], args[1] as *mut i32), // TODO: wait4
        062 => sys_kill(args[0]),
//        072 => sys_fcntl(),
//        074 => sys_fsync(),
//        076 => sys_trunc(),
//        077 => sys_ftrunc(),
        078 => sys_getdirentry(args[0], args[1] as *mut DirEntry),
//        079 => sys_getcwd(),
//        080 => sys_chdir(),
//        082 => sys_rename(),
//        083 => sys_mkdir(),
//        086 => sys_link(),
//        087 => sys_unlink(),
        096 => sys_get_time(), // TODO: sys_gettimeofday
//        097 => sys_getrlimit(),
//        098 => sys_getrusage(),
//        133 => sys_mknod(),
        141 => sys_set_priority(args[0]),
//        160 => sys_setrlimit(),
//        162 => sys_sync(),
//        169 => sys_reboot(),
//        293 => sys_pipe(),

        // for musl: empty impl
        158 => {
            warn!("sys_arch_prctl is unimplemented");
            Ok(0)
        }
        218 => {
            warn!("sys_set_tid_address is unimplemented");
            Ok(0)
        }
        _ => {
            error!("unknown syscall id: {:#x?}, args: {:x?}", id, args);
            crate::trap::error(tf);
        }
    };
    match ret {
        Ok(code) => code,
        Err(err) => -(err as isize),
    }
}

pub type SysResult = Result<isize, SysError>;

#[repr(isize)]
#[derive(Debug)]
pub enum SysError {
    // ucore compatible error code
    // note that ucore_plus use another error code table, which is a modified version of the ones used in linux
    // name conversion E_XXXXX -> SysError::Xxxxx
    // see https://github.com/oscourse-tsinghua/ucore_os_lab/blob/master/labcodes/lab8/libs/error.h
    // we only add current used errors here
    Inval = 3,// Invalid argument, also Invaild fd number.
    Nomem = 4,// Out of memory, also used as no device space in ucore
    Noent = 16,// No such file or directory
    Isdir = 17,// Fd is a directory
    Notdir = 18,// Fd is not a directory
    Xdev = 19,// Cross-device link
    Unimp = 20,// Not implemented
    Exists = 23,// File exists
    Notempty = 24,// Directory is not empty

    #[allow(dead_code)]
    Unspcified = 1,// A really really unknown error.
}

impl From<FsError> for SysError {
    fn from(error: FsError) -> Self {
        match error {
            FsError::NotSupported => SysError::Unimp,
            FsError::NotFile => SysError::Isdir,
            FsError::IsDir => SysError::Isdir,
            FsError::NotDir => SysError::Notdir,
            FsError::EntryNotFound => SysError::Noent,
            FsError::EntryExist => SysError::Exists,
            FsError::NotSameFs => SysError::Xdev,
            FsError::InvalidParam => SysError::Inval,
            FsError::NoDeviceSpace => SysError::Nomem,
            FsError::DirRemoved => SysError::Noent,
            FsError::DirNotEmpty => SysError::Notempty,
            FsError::WrongFs => SysError::Inval,
        }
    }
}
