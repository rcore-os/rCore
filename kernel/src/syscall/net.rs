//! Syscalls for networking

use super::*;
use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};
use crate::process::structs::TcpSocketState;
use core::cmp::min;
use core::mem::size_of;
use smoltcp::socket::*;
use smoltcp::wire::*;

const AF_UNIX: usize = 1;
const AF_INET: usize = 2;

const SOCK_STREAM: usize = 1;
const SOCK_DGRAM: usize = 2;
const SOCK_RAW: usize = 3;
const SOCK_TYPE_MASK: usize = 0xf;

const IPPROTO_IP: usize = 0;
const IPPROTO_ICMP: usize = 1;
const IPPROTO_TCP: usize = 6;

const TCP_SENDBUF: usize = 512 * 1024; // 512K
const TCP_RECVBUF: usize = 512 * 1024; // 512K

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

pub fn sys_socket(domain: usize, socket_type: usize, protocol: usize) -> SysResult {
    info!(
        "socket: domain: {}, socket_type: {}, protocol: {}",
        domain, socket_type, protocol
    );
    let mut proc = process();
    let iface = &*(NET_DRIVERS.read()[0]);
    match domain {
        AF_INET | AF_UNIX => match socket_type & SOCK_TYPE_MASK {
            SOCK_STREAM => {
                let fd = proc.get_free_fd();

                let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; TCP_RECVBUF]);
                let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; TCP_SENDBUF]);
                let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

                let tcp_handle = iface.sockets().add(tcp_socket);
                proc.files.insert(
                    fd,
                    FileLike::Socket(SocketWrapper {
                        handle: tcp_handle,
                        socket_type: SocketType::Tcp(TcpSocketState {
                            local_endpoint: None,
                            is_listening: false,
                        }),
                    }),
                );

                Ok(fd)
            }
            SOCK_DGRAM => {
                let fd = proc.get_free_fd();

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
                        socket_type: SocketType::Udp(UdpSocketState {
                            remote_endpoint: None,
                        }),
                    }),
                );

                Ok(fd)
            }
            SOCK_RAW => {
                let fd = proc.get_free_fd();

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
                Ok(fd)
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

const SOL_SOCKET: usize = 1;
const SO_SNDBUF: usize = 7;
const SO_RCVBUF: usize = 8;
const SO_LINGER: usize = 13;

const TCP_CONGESTION: usize = 13;

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
    proc.memory_set.check_mut_ptr(optlen)?;
    match level {
        SOL_SOCKET => match optname {
            SO_SNDBUF => {
                proc.memory_set.check_mut_array(optval, 4)?;
                unsafe {
                    *(optval as *mut u32) = TCP_SENDBUF as u32;
                    *optlen = 4;
                }
                Ok(0)
            }
            SO_RCVBUF => {
                proc.memory_set.check_mut_array(optval, 4)?;
                unsafe {
                    *(optval as *mut u32) = TCP_RECVBUF as u32;
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

pub fn sys_connect(fd: usize, addr: *const SockAddr, addr_len: usize) -> SysResult {
    info!(
        "sys_connect: fd: {}, addr: {:?}, addr_len: {}",
        fd, addr, addr_len
    );

    let mut proc = process();

    let endpoint = sockaddr_to_endpoint(&mut proc, addr, addr_len)?;

    let wrapper = &mut proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        let iface = &*(NET_DRIVERS.read()[0]);
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

        let temp_port = get_ephemeral_port();

        match socket.connect(endpoint, temp_port) {
            Ok(()) => {
                // avoid deadlock
                drop(socket);
                drop(sockets);

                // wait for connection result
                loop {
                    let iface = &*(NET_DRIVERS.read()[0]);
                    iface.poll();

                    let mut sockets = iface.sockets();
                    let socket = sockets.get::<TcpSocket>(wrapper.handle);
                    if socket.state() == TcpState::SynSent {
                        // still connecting
                        drop(socket);
                        drop(sockets);
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
    } else if let SocketType::Udp(_) = wrapper.socket_type {
        wrapper.socket_type = SocketType::Udp(UdpSocketState {
            remote_endpoint: Some(endpoint),
        });
        Ok(0)
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_write_socket(proc: &mut Process, fd: usize, base: *const u8, len: usize) -> SysResult {
    let iface = &*(NET_DRIVERS.read()[0]);
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
                        Ok(size)
                    }
                    Err(err) => Err(SysError::ENOBUFS),
                }
            } else {
                Err(SysError::ENOBUFS)
            }
        } else {
            Err(SysError::ENOTCONN)
        }
    } else if let SocketType::Udp(ref state) = wrapper.socket_type {
        if let Some(ref remote_endpoint) = state.remote_endpoint {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<UdpSocket>(wrapper.handle);

            if socket.endpoint().port == 0 {
                let v4_src = iface.ipv4_address().unwrap();
                let temp_port = get_ephemeral_port();
                socket
                    .bind(IpEndpoint::new(IpAddress::Ipv4(v4_src), temp_port))
                    .unwrap();
            }

            let slice = unsafe { slice::from_raw_parts(base, len) };
            if socket.is_open() {
                if socket.can_send() {
                    match socket.send_slice(&slice, *remote_endpoint) {
                        Ok(()) => {
                            // avoid deadlock
                            drop(socket);
                            drop(sockets);

                            iface.poll();
                            Ok(len)
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
            Err(SysError::ENOTCONN)
        }
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_read_socket(proc: &mut Process, fd: usize, base: *mut u8, len: usize) -> SysResult {
    let iface = &*(NET_DRIVERS.read()[0]);
    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        spin_and_wait(&[&SOCKET_ACTIVITY], move || {
            iface.poll();
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

            if socket.is_open() {
                let mut slice = unsafe { slice::from_raw_parts_mut(base, len) };
                if let Ok(size) = socket.recv_slice(&mut slice) {
                    if size > 0 {
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        iface.poll();
                        return Some(Ok(size));
                    }
                }
            } else {
                return Some(Err(SysError::ENOTCONN));
            }
            None
        })
    } else if let SocketType::Udp(_) = wrapper.socket_type {
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
                    return Ok(size);
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

pub fn sys_sendto(
    fd: usize,
    buffer: *const u8,
    len: usize,
    flags: usize,
    addr: *const SockAddr,
    addr_len: usize,
) -> SysResult {
    info!(
        "sys_sendto: fd: {} buffer: {:?} len: {} addr: {:?} addr_len: {}",
        fd, buffer, len, addr, addr_len
    );

    let mut proc = process();
    proc.memory_set.check_array(buffer, len)?;

    let endpoint = sockaddr_to_endpoint(&mut proc, addr, addr_len)?;

    let iface = &*(NET_DRIVERS.read()[0]);

    let wrapper = proc.get_socket(fd)?;
    if let SocketType::Raw = wrapper.socket_type {
        let v4_src = iface.ipv4_address().unwrap();
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<RawSocket>(wrapper.handle);

        if let IpAddress::Ipv4(v4_dst) = endpoint.addr {
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

            Ok(len)
        } else {
            unimplemented!("ip type")
        }
    } else if let SocketType::Udp(_) = wrapper.socket_type {
        let v4_src = iface.ipv4_address().unwrap();
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<UdpSocket>(wrapper.handle);

        if socket.endpoint().port == 0 {
            let temp_port = get_ephemeral_port();
            socket
                .bind(IpEndpoint::new(IpAddress::Ipv4(v4_src), temp_port))
                .unwrap();
        }

        let slice = unsafe { slice::from_raw_parts(buffer, len) };

        socket.send_slice(&slice, endpoint).unwrap();

        // avoid deadlock
        drop(socket);
        drop(sockets);
        iface.poll();

        Ok(len)
    } else {
        unimplemented!("socket type")
    }
}

pub fn sys_recvfrom(
    fd: usize,
    buffer: *mut u8,
    len: usize,
    flags: usize,
    addr: *mut SockAddr,
    addr_len: *mut u32,
) -> SysResult {
    info!(
        "sys_recvfrom: fd: {} buffer: {:?} len: {} flags: {} addr: {:?} addr_len: {:?}",
        fd, buffer, len, flags, addr, addr_len
    );

    let mut proc = process();
    proc.memory_set.check_mut_array(buffer, len)?;

    let iface = &*(NET_DRIVERS.read()[0]);

    let wrapper = proc.get_socket(fd)?;
    // TODO: move some part of these into one generic function
    if let SocketType::Raw = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<RawSocket>(wrapper.handle);

            let mut slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
            if let Ok(size) = socket.recv_slice(&mut slice) {
                let packet = Ipv4Packet::new_unchecked(&slice);

                if !addr.is_null() {
                    // FIXME: check size as per sin_family
                    let sockaddr_in = SockAddr::from(IpEndpoint {
                        addr: IpAddress::Ipv4(packet.src_addr()),
                        port: 0,
                    });
                    unsafe {
                        sockaddr_in.write_to(&mut proc, addr, addr_len)?;
                    }
                }

                return Ok(size);
            }

            // avoid deadlock
            drop(socket);
            drop(sockets);
            SOCKET_ACTIVITY._wait()
        }
    } else if let SocketType::Udp(_) = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<UdpSocket>(wrapper.handle);

            let mut slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
            if let Ok((size, endpoint)) = socket.recv_slice(&mut slice) {
                if !addr.is_null() {
                    let sockaddr_in = SockAddr::from(endpoint);
                    unsafe {
                        sockaddr_in.write_to(&mut proc, addr, addr_len)?;
                    }
                }

                return Ok(size);
            }

            // avoid deadlock
            drop(socket);
            drop(sockets);
            SOCKET_ACTIVITY._wait()
        }
    } else if let SocketType::Tcp(_) = wrapper.socket_type {
        loop {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

            let mut slice = unsafe { slice::from_raw_parts_mut(buffer, len) };
            if let Ok(size) = socket.recv_slice(&mut slice) {
                if !addr.is_null() {
                    let sockaddr_in = SockAddr::from(socket.remote_endpoint());
                    unsafe {
                        sockaddr_in.write_to(&mut proc, addr, addr_len)?;
                    }
                }

                return Ok(size);
            }

            // avoid deadlock
            drop(socket);
            drop(sockets);
            SOCKET_ACTIVITY._wait()
        }
    } else {
        unimplemented!("socket type")
    }
}

impl Clone for SocketWrapper {
    fn clone(&self) -> Self {
        let iface = &*(NET_DRIVERS.read()[0]);
        let mut sockets = iface.sockets();
        sockets.retain(self.handle);

        SocketWrapper {
            handle: self.handle.clone(),
            socket_type: self.socket_type.clone(),
        }
    }
}

impl Drop for SocketWrapper {
    fn drop(&mut self) {
        let iface = &*(NET_DRIVERS.read()[0]);
        let mut sockets = iface.sockets();
        sockets.release(self.handle);
        sockets.prune();

        // send FIN immediately when applicable
        drop(sockets);
        iface.poll();
    }
}

pub fn sys_bind(fd: usize, addr: *const SockAddr, addr_len: usize) -> SysResult {
    info!("sys_bind: fd: {} addr: {:?} len: {}", fd, addr, addr_len);
    let mut proc = process();

    let mut endpoint = sockaddr_to_endpoint(&mut proc, addr, addr_len)?;
    if endpoint.port == 0 {
        endpoint.port = get_ephemeral_port();
    }
    info!("sys_bind: fd: {} bind to {}", fd, endpoint);

    let iface = &*(NET_DRIVERS.read()[0]);
    let wrapper = &mut proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        wrapper.socket_type = SocketType::Tcp(TcpSocketState {
            local_endpoint: Some(endpoint),
            is_listening: false,
        });
        Ok(0)
    } else if let SocketType::Udp(_) = wrapper.socket_type {
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<UdpSocket>(wrapper.handle);
        match socket.bind(endpoint) {
            Ok(()) => Ok(0),
            Err(_) => Err(SysError::EINVAL),
        }
    } else {
        Err(SysError::EINVAL)
    }
}

pub fn sys_listen(fd: usize, backlog: usize) -> SysResult {
    info!("sys_listen: fd: {} backlog: {}", fd, backlog);
    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    let iface = &*(NET_DRIVERS.read()[0]);
    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(ref mut tcp_state) = wrapper.socket_type {
        if tcp_state.is_listening {
            // it is ok to listen twice
            Ok(0)
        } else if let Some(local_endpoint) = tcp_state.local_endpoint {
            let mut sockets = iface.sockets();
            let mut socket = sockets.get::<TcpSocket>(wrapper.handle);

            info!("socket {} listening on {:?}", fd, local_endpoint);
            if !socket.is_listening() {
                match socket.listen(local_endpoint) {
                    Ok(()) => {
                        tcp_state.is_listening = true;
                        Ok(0)
                    }
                    Err(err) => Err(SysError::EINVAL),
                }
            } else {
                Ok(0)
            }
        } else {
            Err(SysError::EINVAL)
        }
    } else {
        Err(SysError::EINVAL)
    }
}

pub fn sys_shutdown(fd: usize, how: usize) -> SysResult {
    info!("sys_shutdown: fd: {} how: {}", fd, how);
    let mut proc = process();

    let iface = &*(NET_DRIVERS.read()[0]);
    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        let mut sockets = iface.sockets();
        let mut socket = sockets.get::<TcpSocket>(wrapper.handle);
        socket.close();
        Ok(0)
    } else {
        Err(SysError::EINVAL)
    }
}

pub fn sys_accept(fd: usize, addr: *mut SockAddr, addr_len: *mut u32) -> SysResult {
    info!(
        "sys_accept: fd: {} addr: {:?} addr_len: {:?}",
        fd, addr, addr_len
    );
    // smoltcp tcp sockets do not support backlog
    // open multiple sockets for each connection
    let mut proc = process();

    if !addr.is_null() {
        proc.memory_set.check_mut_ptr(addr_len)?;

        let max_addr_len = unsafe { *addr_len } as usize;
        if max_addr_len < size_of::<SockAddr>() {
            debug!("length too short {}", max_addr_len);
            return Err(SysError::EINVAL);
        }

        proc.memory_set.check_mut_ptr(addr)?;
    }

    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(tcp_state) = wrapper.socket_type.clone() {
        if let Some(endpoint) = tcp_state.local_endpoint {
            loop {
                let iface = &*(NET_DRIVERS.read()[0]);
                let mut sockets = iface.sockets();
                let socket = sockets.get::<TcpSocket>(wrapper.handle);

                if socket.is_active() {
                    let remote_endpoint = socket.remote_endpoint();
                    drop(socket);

                    // move the current one to new_fd
                    // create a new one in fd
                    let new_fd = proc.get_free_fd();

                    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; TCP_RECVBUF]);
                    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; TCP_SENDBUF]);
                    let mut tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);
                    tcp_socket.listen(endpoint).unwrap();

                    let tcp_handle = sockets.add(tcp_socket);

                    let mut orig_socket = proc
                        .files
                        .insert(
                            fd,
                            FileLike::Socket(SocketWrapper {
                                handle: tcp_handle,
                                socket_type: SocketType::Tcp(tcp_state),
                            }),
                        )
                        .unwrap();

                    if let FileLike::Socket(ref mut wrapper) = orig_socket {
                        if let SocketType::Tcp(ref mut state) = wrapper.socket_type {
                            state.is_listening = false;
                        } else {
                            panic!("impossible");
                        }
                    } else {
                        panic!("impossible");
                    }
                    proc.files.insert(new_fd, orig_socket);

                    if !addr.is_null() {
                        let sockaddr_in = SockAddr::from(remote_endpoint);
                        unsafe {
                            sockaddr_in.write_to(&mut proc, addr, addr_len)?;
                        }
                    }

                    drop(sockets);
                    drop(proc);
                    iface.poll();
                    return Ok(new_fd);
                }

                // avoid deadlock
                drop(socket);
                drop(sockets);
                drop(iface);
                SOCKET_ACTIVITY._wait()
            }
        } else {
            Err(SysError::EINVAL)
        }
    } else {
        debug!("bad socket type {:?}", wrapper);
        Err(SysError::EINVAL)
    }
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

    let iface = &*(NET_DRIVERS.read()[0]);
    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(state) = &wrapper.socket_type {
        if let Some(endpoint) = state.local_endpoint {
            let sockaddr_in = SockAddr::from(endpoint);
            unsafe {
                sockaddr_in.write_to(&mut proc, addr, addr_len)?;
            }
            Ok(0)
        } else {
            let mut sockets = iface.sockets();
            let socket = sockets.get::<TcpSocket>(wrapper.handle);
            let endpoint = socket.local_endpoint();
            if endpoint.port != 0 {
                let sockaddr_in = SockAddr::from(socket.local_endpoint());
                unsafe {
                    sockaddr_in.write_to(&mut proc, addr, addr_len)?;
                }
                Ok(0)
            } else {
                Err(SysError::EINVAL)
            }
        }
    } else if let SocketType::Udp(_) = &wrapper.socket_type {
        let mut sockets = iface.sockets();
        let socket = sockets.get::<UdpSocket>(wrapper.handle);
        let endpoint = socket.endpoint();
        if endpoint.port != 0 {
            let sockaddr_in = SockAddr::from(endpoint);
            unsafe {
                sockaddr_in.write_to(&mut proc, addr, addr_len)?;
            }
            Ok(0)
        } else {
            Err(SysError::EINVAL)
        }
    } else {
        Err(SysError::EINVAL)
    }
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

    let iface = &*(NET_DRIVERS.read()[0]);
    let wrapper = proc.get_socket_mut(fd)?;
    if let SocketType::Tcp(_) = wrapper.socket_type {
        let mut sockets = iface.sockets();
        let socket = sockets.get::<TcpSocket>(wrapper.handle);

        if socket.is_open() {
            let remote_endpoint = socket.remote_endpoint();
            let sockaddr_in = SockAddr::from(remote_endpoint);
            unsafe {
                sockaddr_in.write_to(&mut proc, addr, addr_len)?;
            }
            Ok(0)
        } else {
            Err(SysError::EINVAL)
        }
    } else if let SocketType::Udp(state) = &wrapper.socket_type {
        if let Some(endpoint) = state.remote_endpoint {
            let sockaddr_in = SockAddr::from(endpoint);
            unsafe {
                sockaddr_in.write_to(&mut proc, addr, addr_len)?;
            }
            Ok(0)
        } else {
            Err(SysError::EINVAL)
        }
    } else {
        Err(SysError::EINVAL)
    }
}

/// Check socket state
/// return (in, out, err)
pub fn poll_socket(wrapper: &SocketWrapper) -> (bool, bool, bool) {
    let mut input = false;
    let mut output = false;
    let mut err = false;
    if let SocketType::Tcp(state) = wrapper.socket_type.clone() {
        let iface = &*(NET_DRIVERS.read()[0]);
        let mut sockets = iface.sockets();
        let socket = sockets.get::<TcpSocket>(wrapper.handle);

        if state.is_listening && socket.is_active() {
            // a new connection
            input = true;
        } else if !socket.is_open() {
            err = true;
        } else {
            if socket.can_recv() {
                input = true;
            }

            if socket.can_send() {
                output = true;
            }
        }
    } else if let SocketType::Udp(_) = wrapper.socket_type {
        let iface = &*(NET_DRIVERS.read()[0]);
        let mut sockets = iface.sockets();
        let socket = sockets.get::<UdpSocket>(wrapper.handle);

        if socket.can_recv() {
            input = true;
        }

        if socket.can_send() {
            output = true;
        }
    } else {
        unimplemented!()
    }

    (input, output, err)
}

pub fn sys_dup2_socket(proc: &mut Process, wrapper: SocketWrapper, fd: usize) -> SysResult {
    proc.files.insert(fd, FileLike::Socket(wrapper));
    Ok(fd)
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
    proc.memory_set.check_array(addr as *const u8, len)?;
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

        proc.memory_set.check_mut_ptr(addr_len)?;
        let max_addr_len = *addr_len as usize;
        let full_len = match self.family as usize {
            AF_INET => size_of::<u16>() + size_of::<SockAddrIn>(),
            AF_UNIX => return Err(SysError::EINVAL),
            _ => return Err(SysError::EINVAL),
        };

        let written_len = min(max_addr_len, full_len);
        if written_len > 0 {
            proc.memory_set
                .check_mut_array(addr as *mut u8, written_len)?;
            let source = slice::from_raw_parts(&self as *const SockAddr as *const u8, written_len);
            let target = slice::from_raw_parts_mut(addr as *mut u8, written_len);
            target.copy_from_slice(source);
        }
        addr_len.write(full_len as u32);
        return Ok(0);
    }
}
