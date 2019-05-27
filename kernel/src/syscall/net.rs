//! Syscalls for networking

use super::fs::IoVecs;
use super::*;
use crate::fs::FileLike;
use crate::memory::MemorySet;
use crate::net::{
    Endpoint, LinkLevelEndpoint, NetlinkEndpoint, NetlinkSocketState, PacketSocketState,
    RawSocketState, Socket, TcpSocketState, UdpSocketState,
};
use alloc::boxed::Box;
use core::cmp::min;
use core::mem::size_of;
use smoltcp::wire::*;

impl Syscall<'_> {
    pub fn sys_socket(&mut self, domain: usize, socket_type: usize, protocol: usize) -> SysResult {
        let domain = AddressFamily::from(domain as u16);
        let socket_type = SocketType::from(socket_type as u8 & SOCK_TYPE_MASK);
        info!(
            "socket: domain: {:?}, socket_type: {:?}, protocol: {}",
            domain, socket_type, protocol
        );
        let mut proc = self.process();
        let socket: Box<dyn Socket> = match domain {
            AddressFamily::Internet | AddressFamily::Unix => match socket_type {
                SocketType::Stream => Box::new(TcpSocketState::new()),
                SocketType::Datagram => Box::new(UdpSocketState::new()),
                SocketType::Raw => Box::new(RawSocketState::new(protocol as u8)),
                _ => return Err(SysError::EINVAL),
            },
            AddressFamily::Packet => match socket_type {
                SocketType::Raw => Box::new(PacketSocketState::new()),
                _ => return Err(SysError::EINVAL),
            },
            AddressFamily::Netlink => match socket_type {
                SocketType::Raw => Box::new(NetlinkSocketState::new()),
                _ => return Err(SysError::EINVAL),
            },
            _ => return Err(SysError::EAFNOSUPPORT),
        };
        let fd = proc.add_file(FileLike::Socket(socket));
        Ok(fd)
    }

    pub fn sys_setsockopt(
        &mut self,
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
        let mut proc = self.process();
        let data = unsafe { self.vm().check_read_array(optval, optlen)? };
        let socket = proc.get_socket(fd)?;
        socket.setsockopt(level, optname, data)
    }

    pub fn sys_getsockopt(
        &mut self,
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
        let optlen = unsafe { self.vm().check_write_ptr(optlen)? };
        match level {
            SOL_SOCKET => match optname {
                SO_SNDBUF => {
                    let optval = unsafe { self.vm().check_write_ptr(optval as *mut u32)? };
                    *optval = crate::net::TCP_SENDBUF as u32;
                    *optlen = 4;
                    Ok(0)
                }
                SO_RCVBUF => {
                    let optval = unsafe { self.vm().check_write_ptr(optval as *mut u32)? };
                    *optval = crate::net::TCP_RECVBUF as u32;
                    *optlen = 4;
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

    pub fn sys_connect(&mut self, fd: usize, addr: *const SockAddr, addr_len: usize) -> SysResult {
        info!(
            "sys_connect: fd: {}, addr: {:?}, addr_len: {}",
            fd, addr, addr_len
        );

        let mut proc = self.process();
        let endpoint = sockaddr_to_endpoint(&mut self.vm(), addr, addr_len)?;
        let socket = proc.get_socket(fd)?;
        socket.connect(endpoint)?;
        Ok(0)
    }

    pub fn sys_sendto(
        &mut self,
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

        let mut proc = self.process();

        let slice = unsafe { self.vm().check_read_array(base, len)? };
        let endpoint = if addr.is_null() {
            None
        } else {
            let endpoint = sockaddr_to_endpoint(&mut self.vm(), addr, addr_len)?;
            info!("sys_sendto: sending to endpoint {:?}", endpoint);
            Some(endpoint)
        };
        let socket = proc.get_socket(fd)?;
        socket.write(&slice, endpoint)
    }

    pub fn sys_recvfrom(
        &mut self,
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

        let mut proc = self.process();

        let mut slice = unsafe { self.vm().check_write_array(base, len)? };
        let socket = proc.get_socket(fd)?;
        let (result, endpoint) = socket.read(&mut slice);

        if result.is_ok() && !addr.is_null() {
            let sockaddr_in = SockAddr::from(endpoint);
            unsafe {
                sockaddr_in.write_to(&mut self.vm(), addr, addr_len)?;
            }
        }

        result
    }

    pub fn sys_recvmsg(&mut self, fd: usize, msg: *mut MsgHdr, flags: usize) -> SysResult {
        info!("recvmsg: fd: {}, msg: {:?}, flags: {}", fd, msg, flags);
        let mut proc = self.process();
        let hdr = unsafe { self.vm().check_write_ptr(msg)? };
        let mut iovs =
            unsafe { IoVecs::check_and_new(hdr.msg_iov, hdr.msg_iovlen, &self.vm(), true)? };

        let mut buf = iovs.new_buf(true);
        let socket = proc.get_socket(fd)?;
        let (result, endpoint) = socket.read(&mut buf);

        if let Ok(len) = result {
            // copy data to user
            iovs.write_all_from_slice(&buf[..len]);
            let sockaddr_in = SockAddr::from(endpoint);
            unsafe {
                sockaddr_in.write_to(
                    &mut self.vm(),
                    hdr.msg_name,
                    &mut hdr.msg_namelen as *mut u32,
                )?;
            }
        }
        result
    }

    pub fn sys_bind(&mut self, fd: usize, addr: *const SockAddr, addr_len: usize) -> SysResult {
        info!("sys_bind: fd: {} addr: {:?} len: {}", fd, addr, addr_len);
        let mut proc = self.process();

        let endpoint = sockaddr_to_endpoint(&mut self.vm(), addr, addr_len)?;
        info!("sys_bind: fd: {} bind to {:?}", fd, endpoint);

        let socket = proc.get_socket(fd)?;
        socket.bind(endpoint)
    }

    pub fn sys_listen(&mut self, fd: usize, backlog: usize) -> SysResult {
        info!("sys_listen: fd: {} backlog: {}", fd, backlog);
        // smoltcp tcp sockets do not support backlog
        // open multiple sockets for each connection
        let mut proc = self.process();

        let socket = proc.get_socket(fd)?;
        socket.listen()
    }

    pub fn sys_shutdown(&mut self, fd: usize, how: usize) -> SysResult {
        info!("sys_shutdown: fd: {} how: {}", fd, how);
        let mut proc = self.process();

        let socket = proc.get_socket(fd)?;
        socket.shutdown()
    }

    pub fn sys_accept(&mut self, fd: usize, addr: *mut SockAddr, addr_len: *mut u32) -> SysResult {
        info!(
            "sys_accept: fd: {} addr: {:?} addr_len: {:?}",
            fd, addr, addr_len
        );
        // smoltcp tcp sockets do not support backlog
        // open multiple sockets for each connection
        let mut proc = self.process();

        let socket = proc.get_socket(fd)?;
        let (new_socket, remote_endpoint) = socket.accept()?;

        let new_fd = proc.add_file(FileLike::Socket(new_socket));

        if !addr.is_null() {
            let sockaddr_in = SockAddr::from(remote_endpoint);
            unsafe {
                sockaddr_in.write_to(&mut self.vm(), addr, addr_len)?;
            }
        }
        Ok(new_fd)
    }

    pub fn sys_getsockname(
        &mut self,
        fd: usize,
        addr: *mut SockAddr,
        addr_len: *mut u32,
    ) -> SysResult {
        info!(
            "sys_getsockname: fd: {} addr: {:?} addr_len: {:?}",
            fd, addr, addr_len
        );

        let mut proc = self.process();

        if addr.is_null() {
            return Err(SysError::EINVAL);
        }

        let socket = proc.get_socket(fd)?;
        let endpoint = socket.endpoint().ok_or(SysError::EINVAL)?;
        let sockaddr_in = SockAddr::from(endpoint);
        unsafe {
            sockaddr_in.write_to(&mut self.vm(), addr, addr_len)?;
        }
        Ok(0)
    }

    pub fn sys_getpeername(
        &mut self,
        fd: usize,
        addr: *mut SockAddr,
        addr_len: *mut u32,
    ) -> SysResult {
        info!(
            "sys_getpeername: fd: {} addr: {:?} addr_len: {:?}",
            fd, addr, addr_len
        );

        // smoltcp tcp sockets do not support backlog
        // open multiple sockets for each connection
        let mut proc = self.process();

        if addr as usize == 0 {
            return Err(SysError::EINVAL);
        }

        let socket = proc.get_socket(fd)?;
        let remote_endpoint = socket.remote_endpoint().ok_or(SysError::EINVAL)?;
        let sockaddr_in = SockAddr::from(remote_endpoint);
        unsafe {
            sockaddr_in.write_to(&mut self.vm(), addr, addr_len)?;
        }
        Ok(0)
    }
}

impl Process {
    fn get_socket(&mut self, fd: usize) -> Result<&mut Box<dyn Socket>, SysError> {
        match self.get_file_like(fd)? {
            FileLike::Socket(socket) => Ok(socket),
            _ => Err(SysError::EBADF),
        }
    }
}

#[repr(C)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16,
    pub sin_addr: u32,
    pub sin_zero: [u8; 8],
}

#[repr(C)]
pub struct SockAddrUn {
    pub sun_family: u16,
    pub sun_path: [u8; 108],
}

#[repr(C)]
pub struct SockAddrLl {
    pub sll_family: u16,
    pub sll_protocol: u16,
    pub sll_ifindex: u32,
    pub sll_hatype: u16,
    pub sll_pkttype: u8,
    pub sll_halen: u8,
    pub sll_addr: [u8; 8],
}

#[repr(C)]
pub struct SockAddrNl {
    nl_family: u16,
    nl_pad: u16,
    nl_pid: u32,
    nl_groups: u32,
}

#[repr(C)]
pub union SockAddr {
    pub family: u16,
    pub addr_in: SockAddrIn,
    pub addr_un: SockAddrUn,
    pub addr_ll: SockAddrLl,
    pub addr_nl: SockAddrNl,
    pub addr_ph: SockAddrPlaceholder,
}

#[repr(C)]
pub struct SockAddrPlaceholder {
    pub family: u16,
    pub data: [u8; 14],
}

impl From<Endpoint> for SockAddr {
    fn from(endpoint: Endpoint) -> Self {
        if let Endpoint::Ip(ip) = endpoint {
            match ip.addr {
                IpAddress::Ipv4(ipv4) => SockAddr {
                    addr_in: SockAddrIn {
                        sin_family: AddressFamily::Internet.into(),
                        sin_port: u16::to_be(ip.port),
                        sin_addr: u32::to_be(u32::from_be_bytes(ipv4.0)),
                        sin_zero: [0; 8],
                    },
                },
                IpAddress::Unspecified => SockAddr {
                    addr_ph: SockAddrPlaceholder {
                        family: AddressFamily::Unspecified.into(),
                        data: [0; 14],
                    },
                },
                _ => unimplemented!("only ipv4"),
            }
        } else if let Endpoint::LinkLevel(link_level) = endpoint {
            SockAddr {
                addr_ll: SockAddrLl {
                    sll_family: AddressFamily::Packet.into(),
                    sll_protocol: 0,
                    sll_ifindex: link_level.interface_index as u32,
                    sll_hatype: 0,
                    sll_pkttype: 0,
                    sll_halen: 0,
                    sll_addr: [0; 8],
                },
            }
        } else if let Endpoint::Netlink(netlink) = endpoint {
            SockAddr {
                addr_nl: SockAddrNl {
                    nl_family: AddressFamily::Netlink.into(),
                    nl_pad: 0,
                    nl_pid: netlink.port_id,
                    nl_groups: netlink.multicast_groups_mask,
                },
            }
        } else {
            unimplemented!("only ip");
        }
    }
}

/// Convert sockaddr to endpoint
// Check len is long enough
fn sockaddr_to_endpoint(
    vm: &MemorySet,
    addr: *const SockAddr,
    len: usize,
) -> Result<Endpoint, SysError> {
    if len < size_of::<u16>() {
        return Err(SysError::EINVAL);
    }
    let addr = unsafe { vm.check_read_ptr(addr)? };
    if len < addr.len()? {
        return Err(SysError::EINVAL);
    }
    unsafe {
        match AddressFamily::from(addr.family) {
            AddressFamily::Internet => {
                let port = u16::from_be(addr.addr_in.sin_port);
                let addr = IpAddress::from(Ipv4Address::from_bytes(
                    &u32::from_be(addr.addr_in.sin_addr).to_be_bytes()[..],
                ));
                Ok(Endpoint::Ip((addr, port).into()))
            }
            AddressFamily::Unix => Err(SysError::EINVAL),
            AddressFamily::Packet => Ok(Endpoint::LinkLevel(LinkLevelEndpoint::new(
                addr.addr_ll.sll_ifindex as usize,
            ))),
            AddressFamily::Netlink => Ok(Endpoint::Netlink(NetlinkEndpoint::new(
                addr.addr_nl.nl_pid,
                addr.addr_nl.nl_groups,
            ))),
            _ => Err(SysError::EINVAL),
        }
    }
}

impl SockAddr {
    fn len(&self) -> Result<usize, SysError> {
        match AddressFamily::from(unsafe { self.family }) {
            AddressFamily::Internet => Ok(size_of::<SockAddrIn>()),
            AddressFamily::Packet => Ok(size_of::<SockAddrLl>()),
            AddressFamily::Netlink => Ok(size_of::<SockAddrNl>()),
            AddressFamily::Unix => Err(SysError::EINVAL),
            _ => Err(SysError::EINVAL),
        }
    }

    /// Write to user sockaddr
    /// Check mutability for user
    unsafe fn write_to(self, vm: &MemorySet, addr: *mut SockAddr, addr_len: *mut u32) -> SysResult {
        // Ignore NULL
        if addr.is_null() {
            return Ok(0);
        }

        let addr_len = vm.check_write_ptr(addr_len)?;
        let max_addr_len = *addr_len as usize;
        let full_len = self.len()?;

        let written_len = min(max_addr_len, full_len);
        if written_len > 0 {
            let target = vm.check_write_array(addr as *mut u8, written_len)?;
            let source = slice::from_raw_parts(&self as *const SockAddr as *const u8, written_len);
            target.copy_from_slice(source);
        }
        *addr_len = full_len as u32;
        return Ok(0);
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct MsgHdr {
    msg_name: *mut SockAddr,
    msg_namelen: u32,
    msg_iov: *mut IoVec,
    msg_iovlen: usize,
    msg_control: usize,
    msg_controllen: usize,
    msg_flags: usize,
}

enum_with_unknown! {
    /// Address families
    pub doc enum AddressFamily(u16) {
        /// Unspecified
        Unspecified = 0,
        /// Unix domain sockets
        Unix = 1,
        /// Internet IP Protocol
        Internet = 2,
        /// Netlink
        Netlink = 16,
        /// Packet family
        Packet = 17,
    }
}

const SOCK_TYPE_MASK: u8 = 0xf;

enum_with_unknown! {
    /// Socket types
    pub doc enum SocketType(u8) {
        /// Stream
        Stream = 1,
        /// Datagram
        Datagram = 2,
        /// Raw
        Raw = 3,
    }
}

const IPPROTO_IP: usize = 0;
const IPPROTO_ICMP: usize = 1;
const IPPROTO_TCP: usize = 6;

const SOL_SOCKET: usize = 1;
const SO_SNDBUF: usize = 7;
const SO_RCVBUF: usize = 8;
const SO_LINGER: usize = 13;

const TCP_CONGESTION: usize = 13;

const IP_HDRINCL: usize = 3;
