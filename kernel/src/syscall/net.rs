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

fn parse_addr(sockaddr_in: &SockaddrIn, dest: &mut Option<IpAddress>, port: &mut u16) {
    if sockaddr_in.sin_family == AF_INET as u16 {
        *port = u16::from_be(sockaddr_in.sin_port);
        let addr = u32::from_be(sockaddr_in.sin_addr);
        *dest = Some(IpAddress::v4(
            (addr >> 24) as u8,
            ((addr >> 16) & 0xFF) as u8,
            ((addr >> 8) & 0xFF) as u8,
            (addr & 0xFF) as u8,
        ));
    }
}

fn fill_addr(sockaddr_in: &mut SockaddrIn, dest: IpAddress, port: u16) {
    if let IpAddress::Ipv4(ipv4) = dest {
        sockaddr_in.sin_family = AF_INET as u16;
        sockaddr_in.sin_port = u16::to_be(port);
        sockaddr_in.sin_addr = u32::to_be(
            ((ipv4.0[0] as u32) << 24)
                | ((ipv4.0[1] as u32) << 16)
                | ((ipv4.0[2] as u32) << 8)
                | ipv4.0[3] as u32,
        );
    } else {
        unimplemented!("ipv6");
    }
}

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
                proc.files.insert(
                    fd,
                    FileLike::Socket(SocketWrapper {
                        handle: tcp_handle,
                        socket_type: SocketType::Tcp,
                    }),
                );

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
                proc.files.insert(
                    fd,
                    FileLike::Socket(SocketWrapper {
                        handle: raw_handle,
                        socket_type: SocketType::Raw,
                    }),
                );
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
    fn get_socket(&mut self, fd: usize) -> Result<SocketWrapper, SysError> {
        let file = self.files.get_mut(&fd).ok_or(SysError::EBADF)?;
        match file {
            FileLike::Socket(wrapper) => Ok(wrapper.clone()),
            _ => Err(SysError::ENOTSOCK),
        }
    }
}

pub fn sys_connect(fd: usize, addr: *const u8, addrlen: usize) -> SysResult {
    info!(
        "sys_connect: fd: {}, addr: {:?}, addrlen: {}",
        fd, addr, addrlen
    );

    let mut proc = process();
    proc.memory_set.check_ptr(addr)?;

    let mut dest = None;
    let mut port = 0;

    // FIXME: check size as per sin_family
    let sockaddr_in = unsafe { &*(addr as *const SockaddrIn) };
    parse_addr(&sockaddr_in, &mut dest, &mut port);

    if dest == None {
        return Err(SysError::EINVAL);
    }

    let mut proc = process();
    // little hack: kick it forward
    let iface = &mut *NET_DRIVERS.lock()[0];
    iface.poll(&mut proc.sockets);

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Tcp = wrapper.socket_type {
        let mut socket = proc.sockets.get::<TcpSocket>(wrapper.handle);

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
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_write_socket(proc: &mut Process, fd: usize, base: *const u8, len: usize) -> SysResult {
    // little hack: kick it forward
    let iface = &mut *NET_DRIVERS.lock()[0];
    iface.poll(&mut proc.sockets);

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Tcp = wrapper.socket_type {
        let mut socket = proc.sockets.get::<TcpSocket>(wrapper.handle);
        let slice = unsafe { slice::from_raw_parts(base, len) };
        if socket.is_open() {
            if socket.can_send() {
                match socket.send_slice(&slice) {
                    Ok(size) => Ok(size as isize),
                    Err(err) => Err(SysError::ENOBUFS),
                }
            } else {
                Err(SysError::ENOBUFS)
            }
        } else {
            Err(SysError::ECONNREFUSED)
        }
    } else {
        unimplemented!("socket type")
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
    info!(
        "sys_sendto: fd: {} buffer: {:?} len: {} addr: {:?} addr_len: {}",
        fd, buffer, len, addr, addr_len
    );
    let mut proc = process();
    proc.memory_set.check_ptr(addr)?;
    proc.memory_set.check_array(buffer, len)?;

    // little hack: kick it forward
    let iface = &mut *NET_DRIVERS.lock()[0];
    iface.poll(&mut proc.sockets);

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Raw = wrapper.socket_type {
        let mut socket = proc.sockets.get::<RawSocket>(wrapper.handle);

        let mut dest = None;
        let mut port = 0;

        // FIXME: check size as per sin_family
        let sockaddr_in = unsafe { &*(addr as *const SockaddrIn) };
        parse_addr(&sockaddr_in, &mut dest, &mut port);

        if dest == None {
            return Err(SysError::EINVAL);
        } else if let Some(IpAddress::Ipv4(v4_dest)) = dest {
            let slice = unsafe { slice::from_raw_parts(buffer, len) };
            // using 20-byte IPv4 header
            let mut buffer = vec![0u8; len + 20];
            let mut packet = Ipv4Packet::new_unchecked(&mut buffer);
            packet.set_version(4);
            packet.set_header_len(20);
            packet.set_total_len((20 + len) as u16);
            packet.set_protocol(socket.ip_protocol().into());
            packet.set_src_addr(iface.ipv4_address().unwrap());
            packet.set_dst_addr(v4_dest);
            let payload = packet.payload_mut();
            payload.copy_from_slice(slice);
            packet.fill_checksum();

            socket.send_slice(&buffer).unwrap();

            Ok(len as isize)
        } else {
            unimplemented!("ip type")
        }
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_recvfrom(
    fd: usize,
    buffer: *mut u8,
    len: usize,
    flags: usize,
    addr: *mut u8,
    addr_len: *mut usize,
) -> SysResult {
    info!(
        "sys_recvfrom: fd: {} buffer: {:?} len: {} flags: {} addr: {:?} addr_len: {:?}",
        fd, buffer, len, flags, addr, addr_len
    );
    let mut proc = process();

    // little hack: kick it forward
    let iface = &mut *NET_DRIVERS.lock()[0];
    iface.poll(&mut proc.sockets);

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Raw = wrapper.socket_type {
        let mut socket = proc.sockets.get::<RawSocket>(wrapper.handle);


        let mut slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
        match socket.recv_slice(&mut slice) {
            Ok(size) => {
                let mut packet = Ipv4Packet::new_unchecked(&slice);

                // FIXME: check size as per sin_family
                let mut sockaddr_in = unsafe { &mut *(addr as *mut SockaddrIn) };
                fill_addr(&mut sockaddr_in, IpAddress::Ipv4(packet.src_addr()), 0);
                unsafe { *addr_len = size_of::<SockaddrIn>() };

                Ok(size as isize)
            }
            Err(err) => {
                warn!("err {:?}", err);
                Err(SysError::ENOBUFS)
            }
        }
    } else {
        unimplemented!("socket type")
    }
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
