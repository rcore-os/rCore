use crate::thread;
use crate::drivers::NET_DRIVERS;
use smoltcp::wire::*;
use smoltcp::iface::*;
use smoltcp::socket::*;
use alloc::collections::BTreeMap;
use crate::drivers::NetDriver;
use crate::drivers::net::virtio_net::VirtIONetDriver;
use alloc::vec;
use smoltcp::time::Instant;
use core::fmt::Write;

pub extern fn server(_arg: usize) -> ! {
    if NET_DRIVERS.lock().len() < 1 {
        loop {
            thread::yield_now();
        }
    }

    let driver = {
        let ref_driver = &mut *NET_DRIVERS.lock()[0];
        ref_driver.as_any().downcast_ref::<VirtIONetDriver>().unwrap().clone()
    };
    let ethernet_addr = driver.get_mac();
    let ip_addrs = [IpCidr::new(IpAddress::v4(10,0,0,2), 24)];
    let neighbor_cache = NeighborCache::new(BTreeMap::new());
    let mut iface = EthernetInterfaceBuilder::new(driver.clone())
        .ethernet_addr(ethernet_addr)
        .ip_addrs(ip_addrs)
        .neighbor_cache(neighbor_cache)
        .finalize();

    let udp_rx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 64]);
    let udp_tx_buffer = UdpSocketBuffer::new(vec![UdpPacketMetadata::EMPTY], vec![0; 128]);
    let udp_socket = UdpSocket::new(udp_rx_buffer, udp_tx_buffer);

    let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 1024]);
    let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

    let mut sockets = SocketSet::new(vec![]);
    let udp_handle = sockets.add(udp_socket);
    let tcp_handle = sockets.add(tcp_socket);

    loop {
        {
            let timestamp = Instant::from_millis(unsafe { crate::trap::TICK as i64 });
            match iface.poll(&mut sockets, timestamp) {
                Ok(_) => {},
                Err(e) => {
                    println!("poll error: {}", e);
                }
            }

            // udp server
            {
                let mut socket = sockets.get::<UdpSocket>(udp_handle);
                if !socket.is_open() {
                    socket.bind(6969).unwrap();
                }

                let client = match socket.recv() {
                    Ok((_, endpoint)) => {
                        Some(endpoint)
                    }
                    Err(_) => None
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
        }

        thread::yield_now();
    }

}
