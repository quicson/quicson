#[macro_use]
extern crate log;

use std::pin::Pin;

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

pub fn hex_dump(buf: &[u8]) -> String {
    let vec: Vec<String> = buf.iter().map(|b| format!("{:02x}", b)).collect();
    vec.join("")
}
