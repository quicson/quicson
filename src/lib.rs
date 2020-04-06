#[macro_use]
extern crate log;

use std::pin::Pin;
use quiche::Connection;
use std::net::SocketAddr;

const MAX_DATAGRAM_SIZE: usize = 1350;

pub fn send(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<quiche::Connection>>) {
    // Generate outgoing QUIC packets and send them on the UDP socket, until
    // quiche reports that there are no more packets to be sent.
    let mut out = [0; MAX_DATAGRAM_SIZE];
    loop {
        let write = match conn.send(&mut out) {
            Ok(v) => v,
            Err(quiche::Error::Done) => {
                debug!("done writing");
                break;
            }
            Err(e) => {
                error!("send failed: {:?}", e);
                conn.close(false, 0x1, b"fail").ok();
                break;
            }
        };

        if let Err(e) = socket.send(&out[..write]) {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                debug!("send() would block");
                break;
            }
            panic!("send() failed: {:?}", e);
        }
        debug!("written {}", write);
    }
}

pub fn recv(
    socket: &mio::net::UdpSocket,
    conn: &mut Pin<Box<quiche::Connection>>,
    events: &mio::Events,
) {
    // Read incoming UDP packets from the socket and feed them to quiche,
    // until there are no more packets to read.
    let mut buf = [0; 65535];
    loop {
        if events.is_empty() {
            debug!("timed out");
            conn.on_timeout();
            break;
        }

        let len = match socket.recv(&mut buf) {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    debug!("recv() would block");
                    break;
                }
                panic!("recv() failed: {:?}", e);
            }
        };
        debug!("got {} bytes", len);

        // Process potentially coalesced packets.
        let read = match conn.recv(&mut buf[..len]) {
            Ok(v) => v,
            Err(quiche::Error::Done) => {
                debug!("done reading");
                break;
            }
            Err(e) => {
                error!("recv failed: {:?}", e);
                break;
            }
        };
        debug!("processed {} bytes", read);
    }
}

pub fn send_to(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>, peer: &SocketAddr) {
    let mut out = [0; MAX_DATAGRAM_SIZE];
    loop {
        let write = match conn.send(&mut out) {
            Ok(v) => v,
            Err(quiche::Error::Done) => {
                debug!("{} done writing", conn.trace_id());
                break;
            },
            Err(e) => {
                error!("{} send failed: {:?}", conn.trace_id(), e);
                conn.close(false, 0x1, b"fail").ok();
                break;
            },
        };

        if let Err(e) = socket.send_to(&out[..write], peer) {
            if e.kind() == std::io::ErrorKind::WouldBlock {
                debug!("send() would block");
                break;
            }
            panic!("send() failed: {:?}", e);
        }
        debug!("{} written {} bytes", conn.trace_id(), write);
    }
}

pub fn recv_from(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>) -> Option<SocketAddr> {
    let mut peer = None;
    let mut buf = [0; 65535];
    let mut out = [0; MAX_DATAGRAM_SIZE];
    loop {
        let (len, src) = match socket.recv_from(&mut buf) {
            Ok(v) => v,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::WouldBlock {
                    debug!("recv_from() would block");
                    break;
                }
                panic!("recv_from() failed: {:?}", e);
            },
        };
        match peer {
            None => peer = Some(src),
            _ => ()
        }
        let read = match conn.recv(&mut buf[..len]) {
            Ok(v) => v,
            Err(quiche::Error::Done) => {
                debug!("{} done reading", conn.trace_id());
                break;
            },
            Err(e) => {
                error!("{} recv failed: {:?}", conn.trace_id(), e);
                break;
            },
        };
        debug!("{} processed {} bytes", conn.trace_id(), read);
    }
    peer
}

pub fn read_streams(conn: &mut Pin<Box<Connection>>) {
    let mut buf = [0; 65535];
    for s in conn.readable() {
        while let Ok((read, fin)) = conn.stream_recv(s, &mut buf) {
            debug!("received {} bytes", read);
            let stream_buf = &buf[..read];
            debug!("stream {} has {} bytes (fin? {})", s, stream_buf.len(), fin);
            let msg = String::from_utf8(stream_buf.to_vec()).unwrap();
            info!("got msg: {}", msg);
        }
    }
}

pub fn close_conn(conn: &mut Pin<Box<Connection>>) {
    info!("Closing connection");
    conn.close(true, 0x00, b"kthxbye").unwrap();
    info!("Connection closed {:?}", conn.stats());
}

pub fn create_poll(socket: &mio::net::UdpSocket) -> mio::Poll {
    let poll = mio::Poll::new().unwrap();
    poll.register(
        socket,
        mio::Token(0),
        mio::Ready::readable(),
        mio::PollOpt::edge(),
    )
        .unwrap();
    poll
}

pub fn hex_dump(buf: &[u8]) -> String {
    let vec: Vec<String> = buf.iter().map(|b| format!("{:02x}", b)).collect();
    vec.join("")
}
