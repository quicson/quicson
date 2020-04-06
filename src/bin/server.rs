#[macro_use]
extern crate log;

use std::net;
use quiche::{Config, Connection};
use mio::Poll;
use ring::rand::{SystemRandom, SecureRandom};
use std::pin::Pin;
use std::net::SocketAddr;

const MAX_DATAGRAM_SIZE: usize = 1350;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let mut _args = check_args();
    let socket = create_sock("127.0.0.1:4433");
    let poll = quicson::create_poll(&socket);
    let mut config = create_conf();

    let mut conn = accept_conn(&poll, &socket, &mut config);
    let peer = recv_msg(&socket, &mut conn, &poll);
    send_response(&socket, &mut conn, &peer);
    quicson::close_conn(&mut conn);
}

fn accept_conn(poll: &Poll, socket: &mio::net::UdpSocket, config: &mut Config) -> Pin<Box<Connection>> {
    info!("Waiting for connection...");
    let mut scid = [0; quiche::MAX_CONN_ID_LEN];
    SystemRandom::new().fill(&mut scid[..]).unwrap();
    let mut conn = quiche::accept(&scid, None, config).unwrap();
    let mut events = mio::Events::with_capacity(1024);
    while !conn.is_established() {
        poll.poll(&mut events, conn.timeout()).unwrap();
        let peer = quicson::recv_from(socket, &mut conn);
        quicson::send_to(socket, &mut conn, &peer.unwrap());
    }
    info!("Connection established");
    conn
}

fn recv_msg(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>, poll: &mio::Poll) -> SocketAddr {
    let mut events = mio::Events::with_capacity(1024);
    poll.poll(&mut events, conn.timeout()).unwrap();
    let peer = quicson::recv_from(socket, conn);
    quicson::read_streams(conn);
    peer.unwrap()
}

fn send_response(socket: &mio::net::UdpSocket, conn: &mut Pin<Box<Connection>>, peer: &SocketAddr) {
    info!("Sending Hello response");
    conn.stream_send(4, b"Hello from server", true).unwrap();
    quicson::send_to(socket, conn, peer);
}

fn check_args() -> std::env::Args {
    let mut args = std::env::args();
    let cmd = &args.next().unwrap();
    if args.len() != 0 {
        panic!("Usage: {}", cmd);
    }
    args
}

fn create_sock(ip: &str) -> mio::net::UdpSocket {
    let socket = net::UdpSocket::bind(ip).unwrap();
    mio::net::UdpSocket::from_socket(socket).unwrap()
}

fn create_conf() -> Config {
    let mut config = quiche::Config::new(quiche::PROTOCOL_VERSION).unwrap();
    config.load_cert_chain_from_pem_file("cert.crt").unwrap();
    config.load_priv_key_from_pem_file("cert.key").unwrap();
    config.set_application_protos(b"\x05hq-27").unwrap();
    config.set_max_idle_timeout(5000);
    config.set_max_packet_size(MAX_DATAGRAM_SIZE as u64);
    config.set_initial_max_data(10_000_000);
    config.set_initial_max_stream_data_bidi_local(1_000_000);
    config.set_initial_max_stream_data_bidi_remote(1_000_000);
    config.set_initial_max_stream_data_uni(1_000_000);
    config.set_initial_max_streams_bidi(100);
    config.set_initial_max_streams_uni(100);
    config.set_disable_active_migration(true);
    config.enable_early_data();
    config
}
