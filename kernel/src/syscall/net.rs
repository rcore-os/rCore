//! Syscalls for networking

use super::*;

const AF_INET: usize = 2;

const SOCK_STREAM: usize = 1;

pub fn sys_socket(domain: usize, socket_type: usize, protocol: usize) -> SysResult {
    info!("socket: domain: {}, socket_type: {:?}, protocol: {:#x}", domain, socket_type, protocol);
    let mut proc = process();
    match domain {
        AF_INET =>  {
            return match socket_type {
                SOCK_STREAM => {
                    let fd = proc.get_free_inode();

                    Ok(fd as isize)
                }
                _ => {
                    Err(SysError::EINVAL)
                }
            }
        }
        _ => {
            return Err(SysError::EINVAL);
        }
    }
}