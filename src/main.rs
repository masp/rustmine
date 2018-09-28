extern crate mio;
extern crate bytes;
extern crate rustmine;

use rustmine::ConnectionManager;

use mio::*;
use mio::net::{TcpListener};

const LISTENER: Token = Token(9999);

fn main() {
    let addr = "127.0.0.1:25565".parse().unwrap();

    let connection_listener = TcpListener::bind(&addr).unwrap();
    let poll = Poll::new().unwrap();

    poll.register(&connection_listener, LISTENER, Ready::readable(), PollOpt::edge()).unwrap();

    let mut conns = ConnectionManager::new(&poll);
    let mut events = Events::with_capacity(1024);

    loop {
        println!("Waiting for connections...");
        poll.poll(&mut events, None).unwrap();

        for event in events.iter() {
            println!("Found events");
            match event.token() {
                LISTENER => {
                    if event.readiness().is_readable() {
                        if let Ok((mut client_stream, client_addr)) = connection_listener.accept() {
                            println!("Accepted connection from {}", client_addr);
                            let net_player = conns.add_connection(client_stream, client_addr);
                            let handshake_packets = rustmine::parse_read_stream(net_player);
                            if handshake_packets.len() > 0 {
                                for packet in &handshake_packets {
                                    println!("Packet found: {:?}", packet);
                                }
                            }
                        } else {
                            println!("Failed to get connection from client, trying again later...");
                        }
                    }
                }
                token => {
                    if event.readiness().is_readable() {
                        if let Some(mut net_player) = conns.get_connection(token) {
                            println!("Packets received from client: {:?}", net_player.get_socket().peer_addr());
                            let packets = rustmine::parse_read_stream(&mut net_player);
                            if packets.len() > 0 {
                                for packet in &packets {
                                    println!("Packet found: {:?}", packet);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
