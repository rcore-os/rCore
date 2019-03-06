//! Syscalls for networking

use super::*;
use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};
use core::mem::size_of;
use smoltcp::socket::*;
use smoltcp::wire::*;

const AF_INET: usize = 2;

const SOCK_STREAM: usize = 1;
const SOCK_DGRAM: usize = 2;
const SOCK_RAW: usize = 3;

const IPPROTO_IP: usize = 0;
const IPPROTO_ICMP: usize = 1;

fn get_ephemeral_port() -> u16 {
    // TODO selects non-conflict high port
    static mut EPHEMERAL_PORT: u16 = 49152;
    unsafe {
        if EPHEMERAL_PORT == 65535 {
            EPHEMERAL_PORT = 49152;
        } else {
            EPHEMERAL_PORT = EPHEMERAL_PORT + 1;
        }
        EPHEMERAL_PORT
    }
}

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
    let iface = &mut *(NET_DRIVERS.lock()[0]);
    match domain {
        AF_INET => match socket_type {
            SOCK_STREAM => {
                let fd = proc.get_free_inode();

                let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 2048]);
                let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 2048]);
                let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

                let tcp_handle = iface.sockets().add(tcp_socket);
                proc.files.insert(
                    fd,
                    FileLike::Socket(SocketWrapper {
                        handle: tcp_handle,
                        socket_type: SocketType::Tcp(None),
                    }),
                );

                Ok(fd as isize)
            }
            SOCK_DGRAM => {
                let fd = proc.get_free_inode();

                let udp_rx_buffer =
                    UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 2048]);
                let udp_tx_buffer =
                    UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 2048]);
                let udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);

                let udp_handle = iface.sockets().add(udp_socket);
                proc.files.insert(
                    fd,
                    FileLike::Socket(SocketWrapper {
                        handle: udp_handle,
                        socket_type: SocketType::Udp,
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

                let raw_handle = iface.sockets().add(raw_socket);
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

    fn get_socket_mut(&mut self, fd: usize) -> Result<&mut SocketWrapper, SysError> {
        let file = self.files.get_mut(&fd).ok_or(SysError::EBADF)?;
        match file {
            FileLike::Socket(ref mut wrapper) => Ok(wrapper),
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

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        let mut drivers = NET_DRIVERS.lock();
        let iface = &mut *(drivers[0]);
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

        let temp_port = get_ephemeral_port();

        match socket.connect((dest.unwrap(), port), temp_port) {
            Ok(()) => {
                // avoid deadlock
                drop(socket);
                drop(sockets);
                drop(iface);
                drop(drivers);

                // wait for connection result
                loop {
                    let mut drivers = NET_DRIVERS.lock();
                    let iface = &mut *(drivers[0]);
                    iface.poll();

                    let mut sockets = iface.sockets();
                    let mut socket = sockets.get::<TcpSocket>(wrapper.handle);
                    if socket.state() == TcpState::SynSent {
                        // still connecting
                        drop(socket);
                        drop(sockets);
                        drop(iface);
                        drop(drivers);
                        debug!("poll for connection wait");
                        SOCKET_ACTIVITY._wait();
                    } else if socket.state() == TcpState::Established {
                        break Ok(0);
                    } else {
                        break Err(SysError::ECONNREFUSED);
                    }
                }
            }
            Err(_) => Err(SysError::ENOBUFS),
        }
    } else if let SocketType::Udp = wrapper.socket_type {
        // do nothing when only sendto() is used
        Ok(0)
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_write_socket(proc: &mut Process, fd: usize, base: *const u8, len: usize) -> SysResult {
    let iface = &mut *(NET_DRIVERS.lock()[0]);
    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

        let slice = unsafe { slice::from_raw_parts(base, len) };
        if socket.is_open() {
            if socket.can_send() {
                match socket.send_slice(&slice) {
                    Ok(size) => {
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        iface.poll();
                        Ok(size as isize)
                    }
                    Err(err) => Err(SysError::ENOBUFS),
                }
            } else {
                Err(SysError::ENOBUFS)
            }
        } else {
            Err(SysError::ENOTCONN)
        }
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_read_socket(proc: &mut Process, fd: usize, base: *mut u8, len: usize) -> SysResult {
    let iface = &mut *(NET_DRIVERS.lock()[0]);
    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

            if socket.is_open() {
                let mut slice = unsafe { slice::from_raw_parts_mut(base, len) };
                if let Ok(size) = socket.recv_slice(&mut slice) {
                    // avoid deadlock
                    drop(socket);
                    drop(sockets);

                    iface.poll();
                    return Ok(size as isize);
                }
            } else {
                return Err(SysError::ENOTCONN);
            }

            // avoid deadlock
            drop(socket);
            SOCKET_ACTIVITY._wait()
        }
    } else if let SocketType::Udp = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<UdpSocket>(wrapper.handle);

            if socket.is_open() {
                let mut slice = unsafe { slice::from_raw_parts_mut(base, len) };
                if let Ok((size, _)) = socket.recv_slice(&mut slice) {
                    // avoid deadlock
                    drop(socket);
                    drop(sockets);

                    iface.poll();
                    return Ok(size as isize);
                }
            } else {
                return Err(SysError::ENOTCONN);
            }

            // avoid deadlock
            drop(socket);
            SOCKET_ACTIVITY._wait()
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

    let iface = &mut *(NET_DRIVERS.lock()[0]);

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Raw = wrapper.socket_type {
        let v4_src = iface.ipv4_address().unwrap();
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<RawSocket>(wrapper.handle);

        let mut dest = None;
        let mut port = 0;

        // FIXME: check size as per sin_family
        let sockaddr_in = unsafe { &*(addr as *const SockaddrIn) };
        parse_addr(&sockaddr_in, &mut dest, &mut port);

        if dest == None {
            return Err(SysError::EINVAL);
        } else if let Some(IpAddress::Ipv4(v4_dst)) = dest {
            let slice = unsafe { slice::from_raw_parts(buffer, len) };
            // using 20-byte IPv4 header
            let mut buffer = vec![0u8; len + 20];
            let mut packet = Ipv4Packet::new_unchecked(&mut buffer);
            packet.set_version(4);
            packet.set_header_len(20);
            packet.set_total_len((20 + len) as u16);
            packet.set_protocol(socket.ip_protocol().into());
            packet.set_src_addr(v4_src);
            packet.set_dst_addr(v4_dst);
            let payload = packet.payload_mut();
            payload.copy_from_slice(slice);
            packet.fill_checksum();

            socket.send_slice(&buffer).unwrap();

            // avoid deadlock
            drop(socket);
            drop(sockets);
            iface.poll();

            Ok(len as isize)
        } else {
            unimplemented!("ip type")
        }
    } else if let SocketType::Udp = wrapper.socket_type {
        let v4_src = iface.ipv4_address().unwrap();
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<UdpSocket>(wrapper.handle);

        let mut dest = None;
        let mut port = 0;

        // FIXME: check size as per sin_family
        let sockaddr_in = unsafe { &*(addr as *const SockaddrIn) };
        parse_addr(&sockaddr_in, &mut dest, &mut port);

        if dest == None {
            return Err(SysError::EINVAL);
        } else if let Some(dst_addr) = dest {
            if !socket.endpoint().is_specified() {
                let temp_port = get_ephemeral_port();
                socket
                    .bind(IpEndpoint::new(IpAddress::Ipv4(v4_src), temp_port))
                    .unwrap();
            }

            let slice = unsafe { slice::from_raw_parts(buffer, len) };

            socket
                .send_slice(&slice, IpEndpoint::new(dst_addr, port))
                .unwrap();

            // avoid deadlock
            drop(socket);
            drop(sockets);
            iface.poll();

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
    addr_len: *mut u32,
) -> SysResult {
    info!(
        "sys_recvfrom: fd: {} buffer: {:?} len: {} flags: {} addr: {:?} addr_len: {:?}",
        fd, buffer, len, flags, addr, addr_len
    );

    let mut proc = process();
    proc.memory_set.check_mut_array(buffer, len)?;

    if addr as usize != 0 {
        proc.memory_set.check_mut_ptr(addr_len)?;

        let max_addr_len = unsafe { *addr_len } as usize;
        if max_addr_len < size_of::<SockaddrIn>() {
            return Err(SysError::EINVAL);
        }

        proc.memory_set.check_mut_array(addr, max_addr_len)?;
    }

    let iface = &mut *(NET_DRIVERS.lock()[0]);

    let wrapper = proc.get_socket(fd)?;
    // TODO: move some part of these into one generic function
    if let SocketType::Raw = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<RawSocket>(wrapper.handle);

            let mut slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
            if let Ok(size) = socket.recv_slice(&mut slice) {
                let mut packet = Ipv4Packet::new_unchecked(&slice);

                if addr as usize != 0 {
                    // FIXME: check size as per sin_family
                    let mut sockaddr_in = unsafe { &mut *(addr as *mut SockaddrIn) };
                    fill_addr(&mut sockaddr_in, IpAddress::Ipv4(packet.src_addr()), 0);
                    unsafe { *addr_len = size_of::<SockaddrIn>() as u32 };
                }

                return Ok(size as isize);
            }

            // avoid deadlock
            drop(socket);
            SOCKET_ACTIVITY._wait()
        }
    } else if let SocketType::Udp = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<UdpSocket>(wrapper.handle);

            let mut slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
            if let Ok((size, endpoint)) = socket.recv_slice(&mut slice) {
                if addr as usize != 0 {
                    let mut sockaddr_in = unsafe { &mut *(addr as *mut SockaddrIn) };
                    fill_addr(&mut sockaddr_in, endpoint.addr, endpoint.port);
                    unsafe { *addr_len = size_of::<SockaddrIn>() as u32 };
                }

                return Ok(size as isize);
            }

            // avoid deadlock
            drop(socket);
            SOCKET_ACTIVITY._wait()
        }
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_close_socket(proc: &mut Process, fd: usize, handle: SocketHandle) -> SysResult {
    let iface = &mut *(NET_DRIVERS.lock()[0]);
    let mut sockets = iface.sockets();
    sockets.release(handle);
    sockets.prune();
    Ok(0)
}

pub fn sys_bind(fd: usize, addr: *const u8, len: usize) -> SysResult {
    info!(
        "sys_bind: fd: {} addr: {:?} len: {}",
        fd, addr, len
    );
    let mut proc = process();
    proc.memory_set.check_array(addr, len)?;

    if len < size_of::<SockaddrIn>() {
        return Err(SysError::EINVAL);
    }

    let mut host = None;
    let mut port = 0;

    let sockaddr_in = unsafe { &*(addr as *const SockaddrIn) };
    parse_addr(&sockaddr_in, &mut host, &mut port);

    if host == None {
        return Err(SysError::EINVAL);
    }

    let iface = &mut *(NET_DRIVERS.lock()[0]);
    let wrapper = &mut proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        wrapper.socket_type = SocketType::Tcp(Some(IpEndpoint::new(host.unwrap(), port)));
        Ok(0)
    } else {
        Err(SysError::EINVAL)
    }
}

pub fn sys_listen(fd: usize, backlog: usize) -> SysResult {
    info!(
        "sys_listen: fd: {} backlog: {}",
        fd, backlog
    );
    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    let iface = &mut *(NET_DRIVERS.lock()[0]);
    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(Some(endpoint)) = wrapper.socket_type {
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

        match socket.listen(endpoint) {
            Ok(()) => Ok(0),
            Err(_) => Err(SysError::EINVAL),
        }
    } else {
        Err(SysError::EINVAL)
    }
}

pub fn sys_accept(fd: usize, addr: *mut u8, addr_len: *mut u32) -> SysResult {
    info!(
        "sys_accept: fd: {} addr: {:?} addr_len: {:?}",
        fd, addr, addr_len
    );
    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    if addr as usize != 0 {
        proc.memory_set.check_mut_ptr(addr_len)?;

        let max_addr_len = unsafe { *addr_len } as usize;
        if max_addr_len < size_of::<SockaddrIn>() {
            debug!("length too short {}", max_addr_len);
            return Err(SysError::EINVAL);
        }

        proc.memory_set.check_mut_array(addr, max_addr_len)?;
    }

    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(Some(endpoint)) = wrapper.socket_type {
        loop {
            let iface = &mut *(NET_DRIVERS.lock()[0]);
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

            if socket.is_active() {
                let endpoint = socket.remote_endpoint();
                drop(socket);

                // move the current one to new_fd
                // create a new one in fd
                let new_fd = proc.get_free_inode();

                let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 2048]);
                let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 2048]);
                let mut tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
                tcp_socket.listen(endpoint).unwrap();

                let tcp_handle = sockets.add(tcp_socket);
                let orig_handle = proc
                    .files
                    .insert(
                        fd,
                        FileLike::Socket(SocketWrapper {
                            handle: tcp_handle,
                            socket_type: SocketType::Tcp(Some(endpoint)),
                        }),
                    )
                    .unwrap();
                proc.files.insert(new_fd, orig_handle);

                if addr as usize != 0 {
                    let mut sockaddr_in = unsafe { &mut *(addr as *mut SockaddrIn) };
                    fill_addr(&mut sockaddr_in, endpoint.addr, endpoint.port);
                    unsafe { *addr_len = size_of::<SockaddrIn>() as u32 };
                }
                return Ok(new_fd as isize);
            }

            // avoid deadlock
            drop(socket);
            drop(sockets);
            drop(iface);
            SOCKET_ACTIVITY._wait()
        }
    } else {
        debug!("bad socket type {:?}", wrapper);
        Err(SysError::EINVAL)
    }
}

pub fn sys_getsockname(fd: usize, addr: *mut u8, addr_len: *mut u32) -> SysResult {
    info!(
        "sys_getsockname: fd: {} addr: {:?} addr_len: {:?}",
        fd, addr, addr_len
    );

    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    if addr as usize == 0 {
        return Err(SysError::EINVAL);
    }

    proc.memory_set.check_mut_ptr(addr_len)?;

    let max_addr_len = unsafe { *addr_len } as usize;
    if max_addr_len < size_of::<SockaddrIn>() {
        return Err(SysError::EINVAL);
    }

    proc.memory_set.check_mut_array(addr, max_addr_len)?;

    let iface = &mut *(NET_DRIVERS.lock()[0]);
    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(Some(endpoint)) = wrapper.socket_type {
        let mut sockaddr_in = unsafe { &mut *(addr as *mut SockaddrIn) };
        fill_addr(&mut sockaddr_in, endpoint.addr, endpoint.port);
        unsafe { *addr_len = size_of::<SockaddrIn>() as u32 };
        return Ok(0);
    } else {
        Err(SysError::EINVAL)
    }
}