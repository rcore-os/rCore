//! Syscalls for networking

use super::*;
use crate::drivers::SOCKET_ACTIVITY;
use crate::fs::FileLike;
use crate::net::{RawSocketState, Socket, TcpSocketState, UdpSocketState, SOCKETS};
use crate::sync::{MutexGuard, SpinNoIrq, SpinNoIrqLock as Mutex};
use alloc::boxed::Box;
use core::cmp::min;
use core::mem::size_of;
use smoltcp::wire::*;

pub fn sys_socket(domain: usize, socket_type: usize, protocol: usize) -> SysResult {
    info!(
        "socket: domain: {}, socket_type: {}, protocol: {}",
        domain, socket_type, protocol
    );
    let mut proc = process();
    let socket: Box<dyn Socket> = match domain {
        AF_INET | AF_UNIX => match socket_type & SOCK_TYPE_MASK {
            SOCK_STREAM => Box::new(TcpSocketState::new()),
            SOCK_DGRAM => Box::new(UdpSocketState::new()),
            SOCK_RAW => Box::new(RawSocketState::new(protocol as u8)),
            _ => return Err(SysError::EINVAL),
        },
        _ => return Err(SysError::EAFNOSUPPORT),
    };
    let fd = proc.get_free_fd();
    proc.files.insert(fd, FileLike::Socket(socket));
    Ok(fd)
}

pub fn sys_setsockopt(
    fd: usize,
    level: usize,
    optname: usize,
    optval: *const u8,
    optlen: usize,
) -> SysResult {
    info!(
        "setsockopt: fd: {}, level: {}, optname: {}",
        fd, level, optname
    );
    let mut proc = process();
    proc.vm.check_read_array(optval, optlen)?;
    let data = unsafe { slice::from_raw_parts(optval, optlen) };
    let socket = proc.get_socket(fd)?;
    socket.setsockopt(level, optname, data)
}

pub fn sys_getsockopt(
    fd: usize,
    level: usize,
    optname: usize,
    optval: *mut u8,
    optlen: *mut u32,
) -> SysResult {
    info!(
        "getsockopt: fd: {}, level: {}, optname: {} optval: {:?} optlen: {:?}",
        fd, level, optname, optval, optlen
    );
    let proc = process();
    proc.vm.check_write_ptr(optlen)?;
    match level {
        SOL_SOCKET => match optname {
            SO_SNDBUF => {
                proc.vm.check_write_array(optval, 4)?;
                unsafe {
                    *(optval as *mut u32) = crate::net::TCP_SENDBUF as u32;
                    *optlen = 4;
                }
                Ok(0)
            }
            SO_RCVBUF => {
                proc.vm.check_write_array(optval, 4)?;
                unsafe {
                    *(optval as *mut u32) = crate::net::TCP_RECVBUF as u32;
                    *optlen = 4;
                }
                Ok(0)
            }
            _ => Err(SysError::ENOPROTOOPT),
        },
        IPPROTO_TCP => match optname {
            TCP_CONGESTION => Ok(0),
            _ => Err(SysError::ENOPROTOOPT),
        },
        _ => Err(SysError::ENOPROTOOPT),
    }
}

pub fn sys_connect(fd: usize, addr: *const SockAddr, addr_len: usize) -> SysResult {
    info!(
        "sys_connect: fd: {}, addr: {:?}, addr_len: {}",
        fd, addr, addr_len
    );

    let mut proc = process();
    let endpoint = sockaddr_to_endpoint(&mut proc, addr, addr_len)?;
    let socket = proc.get_socket(fd)?;
    socket.connect(endpoint)?;
    Ok(0)
}

pub fn sys_sendto(
    fd: usize,
    base: *const u8,
    len: usize,
    _flags: usize,
    addr: *const SockAddr,
    addr_len: usize,
) -> SysResult {
    info!(
        "sys_sendto: fd: {} base: {:?} len: {} addr: {:?} addr_len: {}",
        fd, base, len, addr, addr_len
    );

    let mut proc = process();
    proc.vm.check_read_array(base, len)?;

    let slice = unsafe { slice::from_raw_parts(base, len) };
    let endpoint = if addr.is_null() {
        None
    } else {
        let endpoint = sockaddr_to_endpoint(&mut proc, addr, addr_len)?;
        info!("sys_sendto: sending to endpoint {:?}", endpoint);
        Some(endpoint)
    };
    let socket = proc.get_socket(fd)?;
    socket.write(&slice, endpoint)
}

pub fn sys_recvfrom(
    fd: usize,
    base: *mut u8,
    len: usize,
    flags: usize,
    addr: *mut SockAddr,
    addr_len: *mut u32,
) -> SysResult {
    info!(
        "sys_recvfrom: fd: {} base: {:?} len: {} flags: {} addr: {:?} addr_len: {:?}",
        fd, base, len, flags, addr, addr_len
    );

    let mut proc = process();
    proc.vm.check_write_array(base, len)?;

    let socket = proc.get_socket(fd)?;
    let mut slice = unsafe { slice::from_raw_parts_mut(base, len) };
    let (result, endpoint) = socket.read(&mut slice);

    if result.is_ok() && !addr.is_null() {
        let sockaddr_in = SockAddr::from(endpoint);
        unsafe {
            sockaddr_in.write_to(&mut proc, addr, addr_len)?;
        }
    }

    result
}

pub fn sys_bind(fd: usize, addr: *const SockAddr, addr_len: usize) -> SysResult {
    info!("sys_bind: fd: {} addr: {:?} len: {}", fd, addr, addr_len);
    let mut proc = process();

    let mut endpoint = sockaddr_to_endpoint(&mut proc, addr, addr_len)?;
    info!("sys_bind: fd: {} bind to {}", fd, endpoint);

    let socket = proc.get_socket(fd)?;
    socket.bind(endpoint)
}

pub fn sys_listen(fd: usize, backlog: usize) -> SysResult {
    info!("sys_listen: fd: {} backlog: {}", fd, backlog);
    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    let socket = proc.get_socket(fd)?;
    socket.listen()
}

pub fn sys_shutdown(fd: usize, how: usize) -> SysResult {
    info!("sys_shutdown: fd: {} how: {}", fd, how);
    let mut proc = process();

    let socket = proc.get_socket(fd)?;
    socket.shutdown()
}

pub fn sys_accept(fd: usize, addr: *mut SockAddr, addr_len: *mut u32) -> SysResult {
    info!(
        "sys_accept: fd: {} addr: {:?} addr_len: {:?}",
        fd, addr, addr_len
    );
    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    let socket = proc.get_socket(fd)?;
    let (new_socket, remote_endpoint) = socket.accept()?;

    let new_fd = proc.get_free_fd();
    proc.files.insert(new_fd, FileLike::Socket(new_socket));

    if !addr.is_null() {
        let sockaddr_in = SockAddr::from(remote_endpoint);
        unsafe {
            sockaddr_in.write_to(&mut proc, addr, addr_len)?;
        }
    }
    Ok(new_fd)
}

pub fn sys_getsockname(fd: usize, addr: *mut SockAddr, addr_len: *mut u32) -> SysResult {
    info!(
        "sys_getsockname: fd: {} addr: {:?} addr_len: {:?}",
        fd, addr, addr_len
    );

    let mut proc = process();

    if addr.is_null() {
        return Err(SysError::EINVAL);
    }

    let socket = proc.get_socket(fd)?;
    let endpoint = socket.endpoint().ok_or(SysError::EINVAL)?;
    let sockaddr_in = SockAddr::from(endpoint);
    unsafe {
        sockaddr_in.write_to(&mut proc, addr, addr_len)?;
    }
    Ok(0)
}

pub fn sys_getpeername(fd: usize, addr: *mut SockAddr, addr_len: *mut u32) -> SysResult {
    info!(
        "sys_getpeername: fd: {} addr: {:?} addr_len: {:?}",
        fd, addr, addr_len
    );

    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    if addr as usize == 0 {
        return Err(SysError::EINVAL);
    }

    let socket = proc.get_socket(fd)?;
    let remote_endpoint = socket.remote_endpoint().ok_or(SysError::EINVAL)?;
    let sockaddr_in = SockAddr::from(remote_endpoint);
    unsafe {
        sockaddr_in.write_to(&mut proc, addr, addr_len)?;
    }
    Ok(0)
}

impl Process {
    fn get_socket(&mut self, fd: usize) -> Result<&mut Box<dyn Socket>, SysError> {
        match self.get_file_like(fd)? {
            FileLike::Socket(socket) => Ok(socket),
            _ => Err(SysError::EBADF),
        }
    }
}

// cancel alignment
#[repr(packed)]
pub struct SockAddrIn {
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

#[repr(C)]
pub struct SockAddrUn {
    sun_path: [u8; 108],
}

#[repr(C)]
pub union SockAddrPayload {
    addr_in: SockAddrIn,
    addr_un: SockAddrUn,
}

#[repr(C)]
pub struct SockAddr {
    family: u16,
    payload: SockAddrPayload,
}

impl From<IpEndpoint> for SockAddr {
    fn from(endpoint: IpEndpoint) -> Self {
        match endpoint.addr {
            IpAddress::Ipv4(ipv4) => SockAddr {
                family: AF_INET as u16,
                payload: SockAddrPayload {
                    addr_in: SockAddrIn {
                        sin_port: u16::to_be(endpoint.port),
                        sin_addr: u32::to_be(u32::from_be_bytes(ipv4.0)),
                        sin_zero: [0; 8],
                    },
                },
            },
            _ => unimplemented!("ipv6"),
        }
    }
}

/// Convert sockaddr to endpoint
// Check len is long enough
fn sockaddr_to_endpoint(
    proc: &mut Process,
    addr: *const SockAddr,
    len: usize,
) -> Result<IpEndpoint, SysError> {
    if len < size_of::<u16>() {
        return Err(SysError::EINVAL);
    }
    proc.vm.check_read_array(addr as *const u8, len)?;
    unsafe {
        match (*addr).family as usize {
            AF_INET => {
                if len < size_of::<u16>() + size_of::<SockAddrIn>() {
                    return Err(SysError::EINVAL);
                }
                let port = u16::from_be((*addr).payload.addr_in.sin_port);
                let addr = IpAddress::from(Ipv4Address::from_bytes(
                    &u32::from_be((*addr).payload.addr_in.sin_addr).to_be_bytes()[..],
                ));
                Ok((addr, port).into())
            }
            AF_UNIX => Err(SysError::EINVAL),
            _ => Err(SysError::EINVAL),
        }
    }
}

impl SockAddr {
    /// Write to user sockaddr
    /// Check mutability for user
    unsafe fn write_to(
        self,
        proc: &mut Process,
        addr: *mut SockAddr,
        addr_len: *mut u32,
    ) -> SysResult {
        // Ignore NULL
        if addr.is_null() {
            return Ok(0);
        }

        proc.vm.check_write_ptr(addr_len)?;
        let max_addr_len = *addr_len as usize;
        let full_len = match self.family as usize {
            AF_INET => size_of::<u16>() + size_of::<SockAddrIn>(),
            AF_UNIX => return Err(SysError::EINVAL),
            _ => return Err(SysError::EINVAL),
        };

        let written_len = min(max_addr_len, full_len);
        if written_len > 0 {
            proc.vm.check_write_array(addr as *mut u8, written_len)?;
            let source = slice::from_raw_parts(&self as *const SockAddr as *const u8, written_len);
            let target = slice::from_raw_parts_mut(addr as *mut u8, written_len);
            target.copy_from_slice(source);
        }
        addr_len.write(full_len as u32);
        return Ok(0);
    }
}

const AF_UNIX: usize = 1;
const AF_INET: usize = 2;

const SOCK_STREAM: usize = 1;
const SOCK_DGRAM: usize = 2;
const SOCK_RAW: usize = 3;
const SOCK_TYPE_MASK: usize = 0xf;

const IPPROTO_IP: usize = 0;
const IPPROTO_ICMP: usize = 1;
const IPPROTO_TCP: usize = 6;

const SOL_SOCKET: usize = 1;
const SO_SNDBUF: usize = 7;
const SO_RCVBUF: usize = 8;
const SO_LINGER: usize = 13;

const TCP_CONGESTION: usize = 13;

const IP_HDRINCL: usize = 3;
