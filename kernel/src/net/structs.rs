use crate::arch::rand;
use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};
use crate::sync::SpinNoIrqLock as Mutex;
use crate::syscall::*;
use crate::util;
use alloc::boxed::Box;
use alloc::fmt::Debug;
use alloc::sync::Arc;
use alloc::vec::Vec;
use bitflags::*;
use core::cmp::min;
use core::mem::size_of;
use core::slice;

use smoltcp::socket::*;
use smoltcp::wire::*;

#[derive(Clone, Debug)]
pub struct LinkLevelEndpoint {
    pub interface_index: usize,
}

impl LinkLevelEndpoint {
    pub fn new(ifindex: usize) -> Self {
        LinkLevelEndpoint {
            interface_index: ifindex,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NetlinkEndpoint {
    pub port_id: u32,
    pub multicast_groups_mask: u32,
}

impl NetlinkEndpoint {
    pub fn new(port_id: u32, multicast_groups_mask: u32) -> Self {
        NetlinkEndpoint {
            port_id,
            multicast_groups_mask,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Endpoint {
    Ip(IpEndpoint),
    LinkLevel(LinkLevelEndpoint),
    Netlink(NetlinkEndpoint),
}

/// Common methods that a socket must have
pub trait Socket: Send + Sync + Debug {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint);
    fn write(&self, data: &[u8], sendto_endpoint: Option<Endpoint>) -> SysResult;
    fn poll(&self) -> (bool, bool, bool); // (in, out, err)
    fn connect(&mut self, endpoint: Endpoint) -> SysResult;
    fn bind(&mut self, _endpoint: Endpoint) -> SysResult {
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
    fn setsockopt(&mut self, _level: usize, _opt: usize, _data: &[u8]) -> SysResult {
        warn!("setsockopt is unimplemented");
        Ok(0)
    }
    fn ioctl(&mut self, _request: usize, _arg1: usize, _arg2: usize, _arg3: usize) -> SysResult {
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
    // no state, only ethernet egress
}

#[derive(Debug, Clone)]
pub struct NetlinkSocketState {
    data: Arc<Mutex<Vec<Vec<u8>>>>,
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
                                debug!("poll for connection wait");
                                SOCKET_ACTIVITY.wait(sockets);
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

    fn bind(&mut self, endpoint: Endpoint) -> SysResult {
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

            drop(socket);
            SOCKET_ACTIVITY.wait(sockets);
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

            drop(socket);
            SOCKET_ACTIVITY.wait(sockets);
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
                            &u32::from_be((*addr).addr_in.sin_addr).to_be_bytes()[..],
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

            drop(socket);
            SOCKET_ACTIVITY.wait(sockets);
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

/// Common structure:
/// | nlmsghdr | ifinfomsg/ifaddrmsg | rtattr | rtattr | rtattr | ... | rtattr
/// All aligned to 4 bytes boundary

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct NetlinkMessageHeader {
    nlmsg_len: u32,                   // length of message including header
    nlmsg_type: u16,                  // message content
    nlmsg_flags: NetlinkMessageFlags, // additional flags
    nlmsg_seq: u32,                   // sequence number
    nlmsg_pid: u32,                   // sending process port id
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct IfaceInfoMsg {
    ifi_family: u16,
    ifi_type: u16,
    ifi_index: u32,
    ifi_flags: u32,
    ifi_change: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct IfaceAddrMsg {
    ifa_family: u8,
    ifa_prefixlen: u8,
    ifa_flags: u8,
    ifa_scope: u8,
    ifa_index: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
struct RouteAttr {
    rta_len: u16,
    rta_type: u16,
}

bitflags! {
    struct NetlinkMessageFlags : u16 {
        const REQUEST = 0x01;
        const MULTI = 0x02;
        const ACK = 0x04;
        const ECHO = 0x08;
        const DUMP_INTR = 0x10;
        const DUMP_FILTERED = 0x20;
        // GET request
        const ROOT = 0x100;
        const MATCH = 0x200;
        const ATOMIC = 0x400;
        const DUMP = 0x100 | 0x200;
        // NEW request
        const REPLACE = 0x100;
        const EXCL = 0x200;
        const CREATE = 0x400;
        const APPEND = 0x800;
        // DELETE request
        const NONREC = 0x100;
        // ACK message
        const CAPPED = 0x100;
        const ACK_TLVS = 0x200;
    }
}

enum_with_unknown! {
    /// Netlink message types
    pub doc enum NetlinkMessageType(u16) {
        /// Nothing
        Noop = 1,
        /// Error
        Error = 2,
        /// End of a dump
        Done = 3,
        /// Data lost
        Overrun = 4,
        /// New link
        NewLink = 16,
        /// Delete link
        DelLink = 17,
        /// Get link
        GetLink = 18,
        /// Set link
        SetLink = 19,
        /// New addr
        NewAddr = 20,
        /// Delete addr
        DelAddr = 21,
        /// Get addr
        GetAddr = 22,
    }
}

enum_with_unknown! {
    /// Route Attr Types
    pub doc enum RouteAttrTypes(u16) {
        /// Unspecified
        Unspecified = 0,
        /// MAC Address
        Address = 1,
        /// Broadcast
        Broadcast = 2,
        /// Interface name
        Ifname = 3,
        /// MTU
        MTU = 4,
        /// Link
        Link = 5,
    }
}

impl NetlinkSocketState {
    pub fn new() -> Self {
        NetlinkSocketState {
            data: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

trait VecExt {
    fn align4(&mut self);
    fn push_ext<T: Sized>(&mut self, data: T);
    fn set_ext<T: Sized>(&mut self, offset: usize, data: T);
}

impl VecExt for Vec<u8> {
    fn align4(&mut self) {
        let len = (self.len() + 3) & !3;
        if len > self.len() {
            self.resize(len, 0);
        }
    }

    fn push_ext<T: Sized>(&mut self, data: T) {
        let bytes =
            unsafe { slice::from_raw_parts(&data as *const T as *const u8, size_of::<T>()) };
        for byte in bytes {
            self.push(*byte);
        }
    }

    fn set_ext<T: Sized>(&mut self, offset: usize, data: T) {
        if self.len() < offset + size_of::<T>() {
            self.resize(offset + size_of::<T>(), 0);
        }
        let bytes =
            unsafe { slice::from_raw_parts(&data as *const T as *const u8, size_of::<T>()) };
        for i in 0..bytes.len() {
            self[offset + i] = bytes[i];
        }
    }
}

impl Socket for NetlinkSocketState {
    fn read(&self, data: &mut [u8]) -> (SysResult, Endpoint) {
        let mut buffer = self.data.lock();
        if buffer.len() > 0 {
            let msg = buffer.remove(0);
            let len = min(msg.len(), data.len());
            data[..len].copy_from_slice(&msg[..len]);
            (
                Ok(len),
                Endpoint::Netlink(NetlinkEndpoint {
                    port_id: 0,
                    multicast_groups_mask: 0,
                }),
            )
        } else {
            (
                Ok(0),
                Endpoint::Netlink(NetlinkEndpoint {
                    port_id: 0,
                    multicast_groups_mask: 0,
                }),
            )
        }
    }

    fn write(&self, data: &[u8], _sendto_endpoint: Option<Endpoint>) -> SysResult {
        if data.len() < size_of::<NetlinkMessageHeader>() {
            return Err(SysError::EINVAL);
        }
        let header = unsafe { &*(data.as_ptr() as *const NetlinkMessageHeader) };
        if header.nlmsg_len as usize > data.len() {
            return Err(SysError::EINVAL);
        }
        let message_type = NetlinkMessageType::from(header.nlmsg_type);
        debug!("type: {:?}", message_type);
        let mut buffer = self.data.lock();
        buffer.clear();
        match message_type {
            NetlinkMessageType::GetLink => {
                let ifaces = NET_DRIVERS.read();
                for i in 0..ifaces.len() {
                    let mut msg = Vec::new();
                    let new_header = NetlinkMessageHeader {
                        nlmsg_len: 0, // to be determined later
                        nlmsg_type: NetlinkMessageType::NewLink.into(),
                        nlmsg_flags: NetlinkMessageFlags::MULTI,
                        nlmsg_seq: header.nlmsg_seq,
                        nlmsg_pid: header.nlmsg_pid,
                    };
                    msg.push_ext(new_header);

                    let if_info = IfaceInfoMsg {
                        ifi_family: AddressFamily::Unspecified.into(),
                        ifi_type: 0,
                        ifi_index: i as u32,
                        ifi_flags: 0,
                        ifi_change: 0,
                    };
                    msg.align4();
                    msg.push_ext(if_info);

                    let mut attrs = Vec::new();

                    let mac_addr = ifaces[i].get_mac();
                    let attr = RouteAttr {
                        rta_len: (mac_addr.as_bytes().len() + size_of::<RouteAttr>()) as u16,
                        rta_type: RouteAttrTypes::Address.into(),
                    };
                    attrs.align4();
                    attrs.push_ext(attr);
                    for byte in mac_addr.as_bytes() {
                        attrs.push(*byte);
                    }

                    let ifname = ifaces[i].get_ifname();
                    let attr = RouteAttr {
                        rta_len: (ifname.as_bytes().len() + size_of::<RouteAttr>()) as u16,
                        rta_type: RouteAttrTypes::Ifname.into(),
                    };
                    attrs.align4();
                    attrs.push_ext(attr);
                    for byte in ifname.as_bytes() {
                        attrs.push(*byte);
                    }

                    msg.align4();
                    msg.append(&mut attrs);

                    msg.align4();
                    msg.set_ext(0, msg.len() as u32);

                    buffer.push(msg);
                }
            }
            NetlinkMessageType::GetAddr => {
                let ifaces = NET_DRIVERS.read();
                for i in 0..ifaces.len() {
                    let ip_addrs = ifaces[i].get_ip_addresses();
                    for j in 0..ip_addrs.len() {
                        let mut msg = Vec::new();
                        let new_header = NetlinkMessageHeader {
                            nlmsg_len: 0, // to be determined later
                            nlmsg_type: NetlinkMessageType::NewAddr.into(),
                            nlmsg_flags: NetlinkMessageFlags::MULTI,
                            nlmsg_seq: header.nlmsg_seq,
                            nlmsg_pid: header.nlmsg_pid,
                        };
                        msg.push_ext(new_header);

                        let family: u16 = AddressFamily::Internet.into();
                        let if_addr = IfaceAddrMsg {
                            ifa_family: family as u8,
                            ifa_prefixlen: ip_addrs[j].prefix_len(),
                            ifa_flags: 0,
                            ifa_scope: 0,
                            ifa_index: i as u32,
                        };
                        msg.align4();
                        msg.push_ext(if_addr);

                        let mut attrs = Vec::new();

                        let ip_addr = ip_addrs[j].address();
                        let attr = RouteAttr {
                            rta_len: (ip_addr.as_bytes().len() + size_of::<RouteAttr>()) as u16,
                            rta_type: RouteAttrTypes::Address.into(),
                        };
                        attrs.align4();
                        attrs.push_ext(attr);
                        for byte in ip_addr.as_bytes() {
                            attrs.push(*byte);
                        }

                        msg.align4();
                        msg.append(&mut attrs);

                        msg.align4();
                        msg.set_ext(0, msg.len() as u32);

                        buffer.push(msg);
                    }
                }
            }
            _ => {}
        }
        let mut msg = Vec::new();
        let new_header = NetlinkMessageHeader {
            nlmsg_len: 0, // to be determined later
            nlmsg_type: NetlinkMessageType::Done.into(),
            nlmsg_flags: NetlinkMessageFlags::MULTI,
            nlmsg_seq: header.nlmsg_seq,
            nlmsg_pid: header.nlmsg_pid,
        };
        msg.push_ext(new_header);
        msg.align4();
        msg.set_ext(0, msg.len() as u32);
        buffer.push(msg);
        Ok(data.len())
    }

    fn poll(&self) -> (bool, bool, bool) {
        unimplemented!()
    }

    fn connect(&mut self, _endpoint: Endpoint) -> SysResult {
        unimplemented!()
    }

    fn bind(&mut self, _endpoint: Endpoint) -> SysResult {
        Ok(0)
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
