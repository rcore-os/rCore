use alloc::sync::Arc;
use core::fmt;

use crate::drivers::{NET_DRIVERS, SOCKET_ACTIVITY};
use crate::process::structs::Process;
use crate::sync::SpinNoIrqLock as Mutex;
use crate::syscall::*;

use smoltcp::socket::*;
use smoltcp::wire::*;

lazy_static! {
    pub static ref SOCKETS: Arc<Mutex<SocketSet<'static, 'static, 'static>>> =
        Arc::new(Mutex::new(SocketSet::new(vec![])));
}

#[derive(Clone, Debug)]
pub struct TcpSocketState {
    pub local_endpoint: Option<IpEndpoint>, // save local endpoint for bind()
    pub is_listening: bool,
}

#[derive(Clone, Debug)]
pub struct UdpSocketState {
    pub remote_endpoint: Option<IpEndpoint>, // remember remote endpoint for connect()
}

#[derive(Clone, Debug)]
pub enum SocketType {
    Raw,
    Tcp(TcpSocketState),
    Udp(UdpSocketState),
    Icmp,
}

#[derive(Debug)]
pub struct SocketWrapper {
    pub handle: SocketHandle,
    pub socket_type: SocketType,
}

pub fn get_ephemeral_port() -> u16 {
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

/// Safety: call this without SOCKETS locked
pub fn poll_ifaces() {
    for iface in NET_DRIVERS.read().iter() {
        iface.poll();
    }
}

impl Drop for SocketWrapper {
    fn drop(&mut self) {
        let mut sockets = SOCKETS.lock();
        sockets.release(self.handle);
        sockets.prune();

        // send FIN immediately when applicable
        drop(sockets);
        poll_ifaces();
    }
}

impl SocketWrapper {
    pub fn write(&self, data: &[u8], sendto_endpoint: Option<IpEndpoint>) -> SysResult {
        if let SocketType::Raw = self.socket_type {
            if let Some(endpoint) = sendto_endpoint {
                // temporary solution
                let iface = &*(NET_DRIVERS.read()[0]);
                let v4_src = iface.ipv4_address().unwrap();
                let mut sockets = SOCKETS.lock();
                let mut socket = sockets.get::<RawSocket>(self.handle);

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
        } else if let SocketType::Tcp(_) = self.socket_type {
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<TcpSocket>(self.handle);

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
        } else if let SocketType::Udp(ref state) = self.socket_type {
            let remote_endpoint = {
                if let Some(ref endpoint) = sendto_endpoint {
                    endpoint
                } else if let Some(ref endpoint) = state.remote_endpoint {
                    endpoint
                } else {
                    return Err(SysError::ENOTCONN);
                }
            };
            let mut sockets = SOCKETS.lock();
            let mut socket = sockets.get::<UdpSocket>(self.handle);

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
        } else {
            unimplemented!("socket type")
        }
    }

    pub fn read(&self, data: &mut [u8]) -> (SysResult, IpEndpoint) {
        if let SocketType::Raw = self.socket_type {
            loop {
                let mut sockets = SOCKETS.lock();
                let mut socket = sockets.get::<RawSocket>(self.handle);

                if let Ok(size) = socket.recv_slice(data) {
                    let packet = Ipv4Packet::new_unchecked(data);

                    return (
                        Ok(size),
                        IpEndpoint {
                            addr: IpAddress::Ipv4(packet.src_addr()),
                            port: 0,
                        },
                    );
                }

                // avoid deadlock
                drop(socket);
                drop(sockets);
                SOCKET_ACTIVITY._wait()
            }
        } else if let SocketType::Tcp(_) = self.socket_type {
            spin_and_wait(&[&SOCKET_ACTIVITY], move || {
                poll_ifaces();
                let mut sockets = SOCKETS.lock();
                let mut socket = sockets.get::<TcpSocket>(self.handle);

                if socket.is_open() {
                    if let Ok(size) = socket.recv_slice(data) {
                        if size > 0 {
                            let endpoint = socket.remote_endpoint();
                            // avoid deadlock
                            drop(socket);
                            drop(sockets);

                            poll_ifaces();
                            return Some((Ok(size), endpoint));
                        }
                    }
                } else {
                    return Some((Err(SysError::ENOTCONN), IpEndpoint::UNSPECIFIED));
                }
                None
            })
        } else if let SocketType::Udp(ref state) = self.socket_type {
            loop {
                let mut sockets = SOCKETS.lock();
                let mut socket = sockets.get::<UdpSocket>(self.handle);

                if socket.is_open() {
                    if let Ok((size, remote_endpoint)) = socket.recv_slice(data) {
                        let endpoint = remote_endpoint;
                        // avoid deadlock
                        drop(socket);
                        drop(sockets);

                        poll_ifaces();
                        return (Ok(size), endpoint);
                    }
                } else {
                    return (Err(SysError::ENOTCONN), IpEndpoint::UNSPECIFIED);
                }

                // avoid deadlock
                drop(socket);
                SOCKET_ACTIVITY._wait()
            }
        } else {
            unimplemented!("socket type")
        }
    }
}
