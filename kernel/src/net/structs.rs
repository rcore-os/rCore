use crate::arch::rand;
use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};
use crate::sync::SpinNoIrqLock as Mutex;
use crate::syscall::*;
use crate::util;
use alloc::boxed::Box;

use smoltcp::socket::*;
use smoltcp::wire::*;

#[derive(Clone, Debug)]
pub struct LinkLevelEndpoint {
    interface_index: usize,
}

impl LinkLevelEndpoint {
    pub fn new(ifindex: usize) -> Self {
        LinkLevelEndpoint {
            interface_index: ifindex,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Endpoint {
    Ip(IpEndpoint),
    LinkLevel(LinkLevelEndpoint),
}

/// Common methods that a socket must have
pub trait Socket: Send + Sync {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint);
    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult;
    fn poll(&self) -> (bool, bool, bool); // (in, out, err)
    fn connect(&mut self, endpoint: Endpoint) -> SysResult;
    fn bind(&mut self, endpoint: Endpoint) -> SysResult {
        Err(SysError::EINVAL)
    }
    fn listen(&mut self) -> SysResult {
        Err(SysError::EINVAL)
    }
    fn shutdown(&self) -> SysResult {
        Err(SysError::EINVAL)
    }
    fn accept(&mut self) -> Result<(Box<dyn Socket>, Endpoint), SysError> {
        Err(SysError::EINVAL)
    }
    fn endpoint(&self) -> Option<Endpoint> {
        None
    }
    fn remote_endpoint(&self) -> Option<Endpoint> {
        None
    }
    fn setsockopt(&mut self, level: usize, opt: usize, data: &[u8]) -> SysResult {
        warn!("setsockopt is unimplemented");
        Ok(0)
    }
    fn ioctl(&mut self, request: usize, arg1: usize, arg2: usize, arg3: usize) -> SysResult {
        warn!("ioctl is unimplemented for this socket");
        Ok(0)
    }
    fn box_clone(&self) -> Box<dyn Socket>;
}

impl Clone for Box<dyn Socket> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

lazy_static! {
    /// Global SocketSet in smoltcp.
    ///
    /// Because smoltcp is a single thread network stack,
    /// every socket operation needs to lock this.
    pub static ref SOCKETS: Mutex<SocketSet<'static, 'static, 'static>> =
        Mutex::new(SocketSet::new(vec![]));
}

#[derive(Debug, Clone)]
pub struct TcpSocketState {
    handle: GlobalSocketHandle,
    local_endpoint: Option<IpEndpoint>, // save local endpoint for bind()
    is_listening: bool,
}

#[derive(Debug, Clone)]
pub struct UdpSocketState {
    handle: GlobalSocketHandle,
    remote_endpoint: Option<IpEndpoint>, // remember remote endpoint for connect()
}

#[derive(Debug, Clone)]
pub struct RawSocketState {
    handle: GlobalSocketHandle,
    header_included: bool,
}

#[derive(Debug, Clone)]
pub struct PacketSocketState {
    // no state
// only ethernet egress
}

/// A wrapper for `SocketHandle`.
/// Auto increase and decrease reference count on Clone and Drop.
#[derive(Debug)]
struct GlobalSocketHandle(SocketHandle);

impl Clone for GlobalSocketHandle {
    fn clone(&self) -> Self {
        SOCKETS.lock().retain(self.0);
        Self(self.0)
    }
}

impl Drop for GlobalSocketHandle {
    fn drop(&mut self) {
        let mut sockets = SOCKETS.lock();
        sockets.release(self.0);
        sockets.prune();

        // send FIN immediately when applicable
        drop(sockets);
        poll_ifaces();
    }
}

impl TcpSocketState {
    pub fn new() -> Self {
        let rx_buffer = TcpSocketBuffer::new(vec![0; TCP_RECVBUF]);
        let tx_buffer = TcpSocketBuffer::new(vec![0; TCP_SENDBUF]);
        let socket = TcpSocket::new(rx_buffer, tx_buffer);
        let handle = GlobalSocketHandle(SOCKETS.lock().add(socket));

        TcpSocketState {
            handle,
            local_endpoint: None,
            is_listening: false,
        }
    }
}

impl Socket for TcpSocketState {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint) {
        spin_and_wait(&[&SOCKET_ACTIVITY], move || {
            poll_ifaces();
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<TcpSocket>(self.handle.0);

            if socket.is_open() {
                if let Ok(size) = socket.recv_slice(data) {
                    if size > 0 {
                        let endpoint = socket.remote_endpoint();
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        poll_ifaces();
                        return Some((Ok(size), Endpoint::Ip(endpoint)));
                    }
                }
            } else {
                return Some((
                    Err(SysError::ENOTCONN),
                    Endpoint::Ip(IpEndpoint::UNSPECIFIED),
                ));
            }
            None
        })
    }

    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);

        if socket.is_open() {
            if socket.can_send() {
                match socket.send_slice(&data) {
                    Ok(size) => {
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        poll_ifaces();
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
    }

    fn poll(&self) -> (bool, bool, bool) {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get::<TcpSocket>(self.handle.0);

        let (mut input, mut output, mut err) = (false, false, false);
        if self.is_listening && socket.is_active() {
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
        (input, output, err)
    }

    fn connect(&mut self, endpoint: Endpoint) -> SysResult {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);

        if let Endpoint::Ip(ip) = endpoint {
            let temp_port = get_ephemeral_port();

            match socket.connect(ip, temp_port) {
                Ok(()) => {
                    // avoid deadlock
                    drop(socket);
                    drop(sockets);

                    // wait for connection result
                    loop {
                        poll_ifaces();

                        let mut sockets = SOCKETS.lock();
                        let socket = sockets.get::<TcpSocket>(self.handle.0);
                        match socket.state() {
                            TcpState::SynSent => {
                                // still connecting
                                drop(socket);
                                drop(sockets);
                                debug!("poll for connection wait");
                                SOCKET_ACTIVITY._wait();
                            }
                            TcpState::Established => {
                                break Ok(0);
                            }
                            _ => {
                                break Err(SysError::ECONNREFUSED);
                            }
                        }
                    }
                }
                Err(_) => Err(SysError::ENOBUFS),
            }
        } else {
            Err(SysError::EINVAL)
        }
    }

    fn bind(&mut self, mut endpoint: Endpoint) -> SysResult {
        if let Endpoint::Ip(mut ip) = endpoint {
            if ip.port == 0 {
                ip.port = get_ephemeral_port();
            }
            self.local_endpoint = Some(ip);
            self.is_listening = false;
            Ok(0)
        } else {
            Err(SysError::EINVAL)
        }
    }

    fn listen(&mut self) -> SysResult {
        if self.is_listening {
            // it is ok to listen twice
            return Ok(0);
        }
        let local_endpoint = self.local_endpoint.ok_or(SysError::EINVAL)?;
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);

        info!("socket listening on {:?}", local_endpoint);
        if socket.is_listening() {
            return Ok(0);
        }
        match socket.listen(local_endpoint) {
            Ok(()) => {
                self.is_listening = true;
                Ok(0)
            }
            Err(_) => Err(SysError::EINVAL),
        }
    }

    fn shutdown(&self) -> SysResult {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<TcpSocket>(self.handle.0);
        socket.close();
        Ok(0)
    }

    fn accept(&mut self) -> Result<(Box<dyn Socket>, Endpoint), SysError> {
        let endpoint = self.local_endpoint.ok_or(SysError::EINVAL)?;
        loop {
            let mut sockets = SOCKETS.lock();
            let socket = sockets.get::<TcpSocket>(self.handle.0);

            if socket.is_active() {
                let remote_endpoint = socket.remote_endpoint();
                drop(socket);

                let new_socket = {
                    let rx_buffer = TcpSocketBuffer::new(vec![0; TCP_RECVBUF]);
                    let tx_buffer = TcpSocketBuffer::new(vec![0; TCP_SENDBUF]);
                    let mut socket = TcpSocket::new(rx_buffer, tx_buffer);
                    socket.listen(endpoint).unwrap();
                    let new_handle = GlobalSocketHandle(sockets.add(socket));
                    let old_handle = ::core::mem::replace(&mut self.handle, new_handle);

                    Box::new(TcpSocketState {
                        handle: old_handle,
                        local_endpoint: self.local_endpoint,
                        is_listening: false,
                    })
                };

                drop(sockets);
                poll_ifaces();
                return Ok((new_socket, Endpoint::Ip(remote_endpoint)));
            }

            // avoid deadlock
            drop(socket);
            drop(sockets);
            SOCKET_ACTIVITY._wait();
        }
    }

    fn endpoint(&self) -> Option<Endpoint> {
        self.local_endpoint
            .clone()
            .map(|e| Endpoint::Ip(e))
            .or_else(|| {
                let mut sockets = SOCKETS.lock();
                let socket = sockets.get::<TcpSocket>(self.handle.0);
                let endpoint = socket.local_endpoint();
                if endpoint.port != 0 {
                    Some(Endpoint::Ip(endpoint))
                } else {
                    None
                }
            })
    }

    fn remote_endpoint(&self) -> Option<Endpoint> {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get::<TcpSocket>(self.handle.0);
        if socket.is_open() {
            Some(Endpoint::Ip(socket.remote_endpoint()))
        } else {
            None
        }
    }

    fn box_clone(&self) -> Box<dyn Socket> {
        Box::new(self.clone())
    }
}

impl UdpSocketState {
    pub fn new() -> Self {
        let rx_buffer = UdpSocketBuffer::new(
            vec![UdpPacketMetadata::EMPTY; UDP_METADATA_BUF],
            vec![0; UDP_RECVBUF],
        );
        let tx_buffer = UdpSocketBuffer::new(
            vec![UdpPacketMetadata::EMPTY; UDP_METADATA_BUF],
            vec![0; UDP_SENDBUF],
        );
        let socket = UdpSocket::new(rx_buffer, tx_buffer);
        let handle = GlobalSocketHandle(SOCKETS.lock().add(socket));

        UdpSocketState {
            handle,
            remote_endpoint: None,
        }
    }
}

#[repr(C)]
struct ArpReq {
    arp_pa: SockAddrPlaceholder,
    arp_ha: SockAddrPlaceholder,
    arp_flags: u32,
    arp_netmask: SockAddrPlaceholder,
    arp_dev: [u8; 16],
}

impl Socket for UdpSocketState {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint) {
        loop {
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<UdpSocket>(self.handle.0);

            if socket.is_open() {
                if let Ok((size, remote_endpoint)) = socket.recv_slice(data) {
                    let endpoint = remote_endpoint;
                    // avoid deadlock
                    drop(socket);
                    drop(sockets);

                    poll_ifaces();
                    return (Ok(size), Endpoint::Ip(endpoint));
                }
            } else {
                return (
                    Err(SysError::ENOTCONN),
                    Endpoint::Ip(IpEndpoint::UNSPECIFIED),
                );
            }

            // avoid deadlock
            drop(socket);
            SOCKET_ACTIVITY._wait()
        }
    }

    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult {
        let remote_endpoint = {
            if let Some(Endpoint::Ip(ref endpoint)) = sendto_endpoint {
                endpoint
            } else if let Some(ref endpoint) = self.remote_endpoint {
                endpoint
            } else {
                return Err(SysError::ENOTCONN);
            }
        };
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<UdpSocket>(self.handle.0);

        if socket.endpoint().port == 0 {
            let temp_port = get_ephemeral_port();
            socket
                .bind(IpEndpoint::new(IpAddress::Unspecified, temp_port))
                .unwrap();
        }

        if socket.can_send() {
            match socket.send_slice(&data, *remote_endpoint) {
                Ok(()) => {
                    // avoid deadlock
                    drop(socket);
                    drop(sockets);

                    poll_ifaces();
                    Ok(data.len())
                }
                Err(err) => Err(SysError::ENOBUFS),
            }
        } else {
            Err(SysError::ENOBUFS)
        }
    }

    fn poll(&self) -> (bool, bool, bool) {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get::<UdpSocket>(self.handle.0);

        let (mut input, mut output, err) = (false, false, false);
        if socket.can_recv() {
            input = true;
        }
        if socket.can_send() {
            output = true;
        }
        (input, output, err)
    }

    fn connect(&mut self, endpoint: Endpoint) -> SysResult {
        if let Endpoint::Ip(ip) = endpoint {
            self.remote_endpoint = Some(ip);
            Ok(0)
        } else {
            Err(SysError::EINVAL)
        }
    }

    fn bind(&mut self, endpoint: Endpoint) -> SysResult {
        let mut sockets = SOCKETS.lock();
        let mut socket = sockets.get::<UdpSocket>(self.handle.0);
        if let Endpoint::Ip(ip) = endpoint {
            match socket.bind(ip) {
                Ok(()) => Ok(0),
                Err(_) => Err(SysError::EINVAL),
            }
        } else {
            Err(SysError::EINVAL)
        }
    }

    fn ioctl(&mut self, request: usize, arg1: usize, arg2: usize, arg3: usize) -> SysResult {
        match request {
            // SIOCGARP
            0x8954 => {
                // FIXME: check addr
                let req = unsafe { &mut *(arg1 as *mut ArpReq) };
                if let AddressFamily::Internet = AddressFamily::from(req.arp_pa.family) {
                    let name = req.arp_dev.as_ptr();
                    let ifname = unsafe { util::from_cstr(name) };
                    let addr = &req.arp_pa as *const SockAddrPlaceholder as *const SockAddr;
                    let addr = unsafe {
                        IpAddress::from(Ipv4Address::from_bytes(
                            &u32::from_be((*addr).payload.addr_in.sin_addr).to_be_bytes()[..],
                        ))
                    };
                    for iface in NET_DRIVERS.read().iter() {
                        if iface.get_ifname() == ifname {
                            debug!("get arp matched ifname {}", ifname);
                            return match iface.get_arp(addr) {
                                Some(mac) => {
                                    // TODO: update flags
                                    req.arp_ha.data[0..6].copy_from_slice(mac.as_bytes());
                                    Ok(0)
                                }
                                None => Err(SysError::ENOENT),
                            };
                        }
                    }
                    Err(SysError::ENOENT)
                } else {
                    Err(SysError::EINVAL)
                }
            }
            _ => Ok(0),
        }
    }

    fn endpoint(&self) -> Option<Endpoint> {
        let mut sockets = SOCKETS.lock();
        let socket = sockets.get::<UdpSocket>(self.handle.0);
        let endpoint = socket.endpoint();
        if endpoint.port != 0 {
            Some(Endpoint::Ip(endpoint))
        } else {
            None
        }
    }

    fn remote_endpoint(&self) -> Option<Endpoint> {
        self.remote_endpoint.clone().map(|e| Endpoint::Ip(e))
    }

    fn box_clone(&self) -> Box<dyn Socket> {
        Box::new(self.clone())
    }
}

impl RawSocketState {
    pub fn new(protocol: u8) -> Self {
        let rx_buffer = RawSocketBuffer::new(
            vec![RawPacketMetadata::EMPTY; RAW_METADATA_BUF],
            vec![0; RAW_RECVBUF],
        );
        let tx_buffer = RawSocketBuffer::new(
            vec![RawPacketMetadata::EMPTY; RAW_METADATA_BUF],
            vec![0; RAW_SENDBUF],
        );
        let socket = RawSocket::new(
            IpVersion::Ipv4,
            IpProtocol::from(protocol),
            rx_buffer,
            tx_buffer,
        );
        let handle = GlobalSocketHandle(SOCKETS.lock().add(socket));

        RawSocketState {
            handle,
            header_included: false,
        }
    }
}

impl Socket for RawSocketState {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint) {
        loop {
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<RawSocket>(self.handle.0);

            if let Ok(size) = socket.recv_slice(data) {
                let packet = Ipv4Packet::new_unchecked(data);

                return (
                    Ok(size),
                    Endpoint::Ip(IpEndpoint {
                        addr: IpAddress::Ipv4(packet.src_addr()),
                        port: 0,
                    }),
                );
            }

            // avoid deadlock
            drop(socket);
            drop(sockets);
            SOCKET_ACTIVITY._wait()
        }
    }

    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult {
        if self.header_included {
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<RawSocket>(self.handle.0);

            match socket.send_slice(&data) {
                Ok(()) => Ok(data.len()),
                Err(_) => Err(SysError::ENOBUFS),
            }
        } else {
            if let Some(Endpoint::Ip(endpoint)) = sendto_endpoint {
                // temporary solution
                let iface = &*(NET_DRIVERS.read()[0]);
                let v4_src = iface.ipv4_address().unwrap();
                let mut sockets = SOCKETS.lock();
                let mut socket = sockets.get::<RawSocket>(self.handle.0);

                if let IpAddress::Ipv4(v4_dst) = endpoint.addr {
                    let len = data.len();
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
                    payload.copy_from_slice(data);
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
            } else {
                Err(SysError::ENOTCONN)
            }
        }
    }

    fn poll(&self) -> (bool, bool, bool) {
        unimplemented!()
    }

    fn connect(&mut self, _endpoint: Endpoint) -> SysResult {
        unimplemented!()
    }

    fn box_clone(&self) -> Box<dyn Socket> {
        Box::new(self.clone())
    }

    fn setsockopt(&mut self, level: usize, opt: usize, data: &[u8]) -> SysResult {
        match (level, opt) {
            (IPPROTO_IP, IP_HDRINCL) => {
                if let Some(arg) = data.first() {
                    self.header_included = *arg > 0;
                    debug!("hdrincl set to {}", self.header_included);
                }
            }
            _ => {}
        }
        Ok(0)
    }
}

impl PacketSocketState {
    pub fn new() -> Self {
        PacketSocketState {}
    }
}

impl Socket for PacketSocketState {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint) {
        unimplemented!()
    }

    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult {
        if let Some(Endpoint::LinkLevel(endpoint)) = sendto_endpoint {
            let ifaces = NET_DRIVERS.read();
            match ifaces[endpoint.interface_index].send(data) {
                Some(len) => Ok(len),
                None => Err(SysError::ENOBUFS),
            }
        } else {
            Err(SysError::ENOTCONN)
        }
    }

    fn poll(&self) -> (bool, bool, bool) {
        unimplemented!()
    }

    fn connect(&mut self, _endpoint: Endpoint) -> SysResult {
        unimplemented!()
    }

    fn box_clone(&self) -> Box<dyn Socket> {
        Box::new(self.clone())
    }
}

fn get_ephemeral_port() -> u16 {
    // TODO selects non-conflict high port
    static mut EPHEMERAL_PORT: u16 = 0;
    unsafe {
        if EPHEMERAL_PORT == 0 {
            EPHEMERAL_PORT = (49152 + rand::rand() % (65536 - 49152)) as u16;
        }
        if EPHEMERAL_PORT == 65535 {
            EPHEMERAL_PORT = 49152;
        } else {
            EPHEMERAL_PORT = EPHEMERAL_PORT + 1;
        }
        EPHEMERAL_PORT
    }
}

/// Safety: call this without SOCKETS locked
fn poll_ifaces() {
    for iface in NET_DRIVERS.read().iter() {
        iface.poll();
    }
}

pub const TCP_SENDBUF: usize = 512 * 1024; // 512K
pub const TCP_RECVBUF: usize = 512 * 1024; // 512K

const UDP_METADATA_BUF: usize = 1024;
const UDP_SENDBUF: usize = 64 * 1024; // 64K
const UDP_RECVBUF: usize = 64 * 1024; // 64K

const RAW_METADATA_BUF: usize = 1024;
const RAW_SENDBUF: usize = 64 * 1024; // 64K
const RAW_RECVBUF: usize = 64 * 1024; // 64K
