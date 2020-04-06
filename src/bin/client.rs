// Copyright (C) 2018-2019, Cloudflare, Inc.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are
// met:
//
//     * Redistributions of source code must retain the above copyright notice,
//       this list of conditions and the following disclaimer.
//
//     * Redistributions in binary form must reproduce the above copyright
//       notice, this list of conditions and the following disclaimer in the
//       documentation and/or other materials provided with the distribution.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS
// IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO,
// THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
// PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL,
// EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO,
// PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
// PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF
// LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING
// NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
// SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

#[macro_use]
extern crate log;

use std::net::ToSocketAddrs;
use std::pin::Pin;

use quiche::Connection;
use ring::rand::*;
use url::Url;

const MAX_DATAGRAM_SIZE: usize = 1350;
const STREAM_ID: u64 = 4;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let mut args = get_args();
    let url = url::Url::parse(&args.next().unwrap()).unwrap();
    let socket = create_sock(&url);
    let poll = quicson::create_poll(&socket);
    let mut conn = create_conn(&url);

    establish_conn(&socket, &mut conn, &poll);
    send_msg(&socket, &mut conn);
    recv_msg(&socket, &mut conn, &poll);
    quicson::close_conn(&mut conn);
}

fn get_args() -> std::env::Args {
    let mut args = std::env::args();
    let cmd = &args.next().unwrap();
    if args.len() != 1 {
        panic!("Usage: {} URL", cmd);
    }
    args
}

fn establish_conn(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>, poll: &mio::Poll) {
    info!("Establishing connection...");
    // send initial packet
    quicson::send(socket, conn);
    let mut events = mio::Events::with_capacity(1024);
    while !conn.is_established() {
        poll.poll(&mut events, conn.timeout()).unwrap();
        quicson::recv(&socket, conn, &events);
        quicson::send(&socket, conn);
    }
    info!("Connection established");
    // TODO: why do I need these two lines below?
    poll.poll(&mut events, conn.timeout()).unwrap();
    quicson::recv(&socket, conn, &events);
}

fn send_msg(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>) {
    info!("Sending Hello");
    conn.stream_send(STREAM_ID, b"Hello from client", true).unwrap();
    quicson::send(&socket, conn);
}

fn recv_msg(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>, poll: &mio::Poll) {
    let mut events = mio::Events::with_capacity(1024);
    poll.poll(&mut events, conn.timeout()).unwrap();
    quicson::recv(socket, conn, &events);
    quicson::read_streams(conn);
    quicson::send(&socket, conn);
}

fn create_sock(url: &Url) -> mio::net::UdpSocket {
    let peer_addr = url.to_socket_addrs().unwrap().next().unwrap();
    // Bind to INADDR_ANY or IN6ADDR_ANY depending on the IP family of the
    // server address. This is needed on macOS and BSD variants that don't
    // support binding to IN6ADDR_ANY for both v4 and v6.
    let bind_addr = match peer_addr {
        std::net::SocketAddr::V4(_) => "0.0.0.0:0",
        std::net::SocketAddr::V6(_) => "[::]:0",
    };
    let socket = std::net::UdpSocket::bind(bind_addr).unwrap();
    socket.connect(peer_addr).unwrap();
    let socket = mio::net::UdpSocket::from_socket(socket).unwrap();
    info!("local addr: {:}, peer addr: {:}", socket.local_addr().unwrap(), peer_addr);
    socket
}

fn create_conn(url: &Url) -> Pin<Box<Connection>> {
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap();
    config.verify_peer(false); // do not set this in production
    config
        .set_application_protos(b"\x05hq-27\x05hq-25\x05hq-24\x05hq-23\x08http/0.9")
        .unwrap();
    config.set_max_idle_timeout(5000);
    config.set_max_packet_size(MAX_DATAGRAM_SIZE as u64);
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);
    config.set_disable_active_migration(true);

    let mut scid = [0; quiche::MAX_CONN_ID_LEN];
    SystemRandom::new().fill(&mut scid[..]).unwrap();
    info!("scid: {}", quicson::hex_dump(&scid));
    quiche::connect(url.domain(), &scid, &mut config).unwrap()
}
