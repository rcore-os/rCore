//! Syscalls for networking

use super::*;
use crate::drivers::NET_DRIVERS;
use core::mem::size_of;
use smoltcp::socket::*;
use smoltcp::wire::*;

const AF_INET: usize = 2;

const SOCK_STREAM: usize = 1;
const SOCK_DGRAM: usize = 2;
const SOCK_RAW: usize = 3;

const IPPROTO_IP: usize = 0;
const IPPROTO_ICMP: usize = 1;

pub fn sys_socket(domain: usize, socket_type: usize, protocol: usize) -> SysResult {
    info!(
        "socket: domain: {}, socket_type: {}, protocol: {}",
        domain, socket_type, protocol
    );
    let mut proc = process();
    match domain {
        AF_INET => match socket_type {
            SOCK_STREAM => {
                let fd = proc.get_free_inode();

                let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 2048]);
                let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 2048]);
                let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

                let tcp_handle = proc.sockets.add(tcp_socket);
                proc.files.insert(fd, FileLike::Socket(tcp_handle));

                Ok(fd as isize)
            }
            SOCK_RAW => {
                let fd = proc.get_free_inode();

                let raw_rx_buffer =
                    RawSocketBuffer::new(vec![RawPacketMetadata::EMPTY; 2], vec![0; 2048]);
                let raw_tx_buffer =
                    RawSocketBuffer::new(vec![RawPacketMetadata::EMPTY; 2], vec![0; 2048]);
                let raw_socket = RawSocket::new(
                    IpVersion::Ipv4,
                    IpProtocol::from(protocol as u8),
                    raw_rx_buffer,
                    raw_tx_buffer,
                );

                let raw_handle = proc.sockets.add(raw_socket);
                proc.files.insert(fd, FileLike::Socket(raw_handle));
                Ok(fd as isize)
            }
            _ => Err(SysError::EINVAL),
        },
        _ => Err(SysError::EAFNOSUPPORT),
    }
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
    warn!("sys_setsockopt is unimplemented");
    Ok(0)
}

#[repr(C)]
struct SockaddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

impl Process {
    fn get_handle(&mut self, fd: usize) -> Result<SocketHandle, SysError> {
        let file = self.files.get_mut(&fd).ok_or(SysError::EBADF)?;
        match file {
            FileLike::Socket(handle) => Ok(handle.clone()),
            _ => Err(SysError::ENOTSOCK),
        }
    }
}

pub fn sys_connect(fd: usize, addr: *const u8, addrlen: usize) -> SysResult {
    info!(
        "sys_connect: fd: {}, addr: {:?}, addrlen: {}",
        fd, addr, addrlen
    );

    let mut dest = None;
    let mut port = 0;
    if addrlen == size_of::<SockaddrIn>() {
        let sockaddr_in = unsafe { &*(addr as *const SockaddrIn) };
        port = ((sockaddr_in.sin_port & 0xFF) << 8) | (sockaddr_in.sin_port >> 8);
        dest = Some(IpAddress::v4(
            (sockaddr_in.sin_addr & 0xFF) as u8,
            ((sockaddr_in.sin_addr >> 8) & 0xFF) as u8,
            ((sockaddr_in.sin_addr >> 16) & 0xFF) as u8,
            (sockaddr_in.sin_addr >> 24) as u8,
        ));
    }

    if dest == None {
        return Err(SysError::EINVAL);
    }

    let mut proc = process();
    // little hack: kick it forward
    let iface = &mut *NET_DRIVERS.lock()[0];
    iface.poll(&mut proc.sockets);

    // TODO: check its type
    let tcp_handle = proc.get_handle(fd)?;
    let mut socket = proc.sockets.get::<TcpSocket>(tcp_handle);

    // TODO selects non-conflict high port
    static mut EPHEMERAL_PORT: u16 = 49152;
    let temp_port = unsafe {
        if EPHEMERAL_PORT == 65535 {
            EPHEMERAL_PORT = 49152;
        } else {
            EPHEMERAL_PORT = EPHEMERAL_PORT + 1;
        }
        EPHEMERAL_PORT
    };

    match socket.connect((dest.unwrap(), port), temp_port) {
        Ok(()) => Ok(0),
        Err(_) => Err(SysError::EISCONN),
    }
}

pub fn sys_write_socket(
    proc: &mut Process,
    fd: usize,
    base: *const u8,
    len: usize,
) -> SysResult {
    // little hack: kick it forward
    let iface = &mut *NET_DRIVERS.lock()[0];
    iface.poll(&mut proc.sockets);

    // TODO: check its type
    let tcp_handle = proc.get_handle(fd)?;
    let mut socket = proc.sockets.get::<TcpSocket>(tcp_handle);
    let slice = unsafe { slice::from_raw_parts(base, len) };
    if socket.is_open() {
        if socket.can_send() {
            match socket.send_slice(&slice) {
                Ok(size) => Ok(size as isize),
                Err(err) =>  Err(SysError::ENOBUFS)
            }
        } else {
            Err(SysError::ENOBUFS)
        }
    } else {
        Err(SysError::ECONNREFUSED)
    }
}

pub fn sys_select(
    fd: usize,
    inp: *const u8,
    outp: *const u8,
    exp: *const u8,
    tvp: *const u8,
) -> SysResult {
    info!("sys_select: fd: {}", fd);
    warn!("sys_select is unimplemented");
    Err(SysError::EINVAL)
}

pub fn sys_sendto(
    fd: usize,
    buffer: *const u8,
    len: usize,
    flags: usize,
    addr: *const u8,
    addr_len: usize,
) -> SysResult {
    info!("sys_sendto: fd: {} buffer: {:?} len: {}", fd, buffer, len);
    warn!("sys_sendto is unimplemented");
    Err(SysError::EINVAL)
}

pub fn sys_recvfrom(
    fd: usize,
    buffer: *mut u8,
    len: usize,
    flags: usize,
    addr: *const u8,
    addr_len: usize,
) -> SysResult {
    info!("sys_recvfrom: fd: {} buffer: {:?} len: {}", fd, buffer, len);
    warn!("sys_recvfrom is unimplemented");
    Err(SysError::EINVAL)
}

pub fn sys_close_socket(proc: &mut Process, fd: usize, handle: SocketHandle) -> SysResult {
    let mut socket = proc.sockets.remove(handle);
    match socket {
        Socket::Tcp(ref mut tcp_socket) => {
            tcp_socket.close();
        }
        _ => {}
    }

    Ok(0)
}