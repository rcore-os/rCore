//! Kernel shell

use crate::fs::ROOT_INODE;
use crate::process::*;
use alloc::string::String;
use alloc::vec::Vec;

/// Spawn shell as init process
pub fn add_user_shell() {
    // the busybox of alpine linux can not transfer env vars into child process
    // Now we use busybox from
    // https://raw.githubusercontent.com/docker-library/busybox/82bc0333a9ae148fbb4246bcbff1487b3fc0c510/musl/busybox.tar.xz -O busybox.tar.xz
    // This one can transfer env vars!
    // Why???

    #[cfg(target_arch = "mips")]
    let init_shell = "/rust/sh"; //from docker-library

    #[cfg(not(target_arch = "mips"))]
    let init_shell = "/busybox"; //from docker-library

    #[cfg(target_arch = "x86_64")]
    let init_envs: Vec<String> =
        vec!["PATH=/usr/sbin:/usr/bin:/sbin:/bin:/usr/x86_64-alpine-linux-musl/bin".into()];

    #[cfg(not(target_arch = "x86_64"))]
    let init_envs = Vec::new();

    let init_args: Vec<String> = vec!["busybox".into(), "ash".into()];

    if let Ok(inode) = ROOT_INODE.lookup(init_shell) {
        let thread = Thread::new_user(&inode, init_shell, init_args, init_envs);
        spawn(thread);
    } else {
        todo!()
    }
}
