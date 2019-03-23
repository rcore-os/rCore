use crate::drivers::NET_DRIVERS;
use crate::net::SOCKETS;
use crate::thread;
use alloc::vec;
use core::fmt::Write;
use smoltcp::socket::*;

pub extern "C" fn server(_arg: usize) -> ! {
    if NET_DRIVERS.read().len() < 1 {
        loop {
            thread::yield_now();
        }
    }

    let udp_rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 64]);
    let udp_tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 128]);
    let udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);

    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

    let tcp2_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp2_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp2_socket = TcpSocket::new(tcp2_rx_buffer, tcp2_tx_buffer);

    let mut sockets = SOCKETS.lock();
    let udp_handle = sockets.add(udp_socket);
    let tcp_handle = sockets.add(tcp_socket);
    let tcp2_handle = sockets.add(tcp2_socket);
    drop(sockets);

    loop {
        {
            let mut sockets = SOCKETS.lock();

            // udp server
            {
                let mut socket = sockets.get::<UdpSocket>(udp_handle);
                if !socket.is_open() {
                    socket.bind(6969).unwrap();
                }

                let client = match socket.recv() {
                    Ok((_, endpoint)) => Some(endpoint),
                    Err(_) => None,
                };
                if let Some(endpoint) = client {
                    let hello = b"hello\n";
                    socket.send_slice(hello, endpoint).unwrap();
                }
            }

            // simple http server
            {
                let mut socket = sockets.get::<TcpSocket>(tcp_handle);
                if !socket.is_open() {
                    socket.listen(80).unwrap();
                }

                if socket.can_send() {
                    write!(socket, "HTTP/1.1 200 OK\r\nServer: rCore\r\nContent-Length: 13\r\nContent-Type: text/html\r\nConnection: Closed\r\n\r\nHello, world!\r\n").unwrap();
                    socket.close();
                }
            }

            // simple tcp server that just eats everything
            {
                let mut socket = sockets.get::<TcpSocket>(tcp2_handle);
                if !socket.is_open() {
                    socket.listen(2222).unwrap();
                }

                if socket.can_recv() {
                    let mut data = [0u8; 2048];
                    let size = socket.recv_slice(&mut data).unwrap();
                }
            }
        }

        thread::yield_now();
    }
}
