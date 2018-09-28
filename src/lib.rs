#![feature(read_initializer)]

extern crate mio;
extern crate bytes;
extern crate byteorder;

use std::collections::{HashMap};
use std::vec::Vec;

use mio::*;
use mio::net::{TcpStream};
use bytes::*;
use std::net::SocketAddr;
use byteorder::*;
use std::io::Error;
use std::io::Read;
use std::io::ErrorKind;
use std::io::Result;
use std::io::BufReader;
use std::io::Cursor;

mod protocol;

#[derive(Debug)]
pub struct Packet {
    packet_length: i32,
    packet_id: i32,
    data: Bytes,
}

pub struct NetworkPlayer {
    source_ip: SocketAddr,
    socket: TcpStream,
}

impl NetworkPlayer {
    pub fn new(source: SocketAddr, socket: TcpStream) -> NetworkPlayer {
        NetworkPlayer {
            source_ip: source,
            socket,
        }
    }

    pub fn get_socket(&self) -> &TcpStream {
        &self.socket
    }

    pub fn get_socket_mut(&mut self) -> &mut TcpStream {
        &mut self.socket
    }

    pub fn get_source_addr(&self) -> &SocketAddr {
        &self.source_ip
    }
}

pub struct ConnectionManager<'a> {
    connections: HashMap<Token, NetworkPlayer>,
    curr_token: usize,
    poll: &'a Poll,
}

impl<'a> ConnectionManager<'a> {
    pub fn new(poll: &'a Poll) -> ConnectionManager {
        ConnectionManager {
            connections: HashMap::new(),
            curr_token: 0,
            poll,
        }
    }

    pub fn add_connection(&mut self, socket: TcpStream, addr: SocketAddr) -> &mut NetworkPlayer {
        let net_player = NetworkPlayer::new(addr, socket);
        let token = Token(self.curr_token);
        self.connections.insert(token, net_player);

        let net_player = self.connections.get_mut(&token).unwrap();
        self.poll.register(net_player.get_socket_mut(),
                           token,
                           Ready::readable() | Ready::writable(),
                           PollOpt::edge()).unwrap();
        self.curr_token += 1;
        net_player
    }

    pub fn get_connection(&mut self, token: Token) -> Option<&mut NetworkPlayer> {
        self.connections.get_mut(&token)
    }
}



pub fn parse_read_stream(player: &mut NetworkPlayer) -> Vec<Packet> {
    let socket = player.get_socket_mut();

    let mut buf = BufReader::new(socket);
    let mut data: Vec<u8> = Vec::with_capacity(1024);
    let bytes_read = read_to_end(&mut buf, &mut data).unwrap();
    println!("{} bytes read", bytes_read);
    let data = Bytes::from(data);
    let mut cursor = Cursor::new(data);

    let mut packets: Vec<Packet> = Vec::new();
    while let Ok(packet) = consume_packet(&mut cursor) {
        packets.push(packet);
    }

    packets
}

pub fn consume_packet(bytes: &mut Cursor<Bytes>) -> Result<Packet> {
    let (packet_length, _) = bytes.read_var_int()?;
    let (packet_id, id_size) = bytes.read_var_int()?;
    let mut packet_data: Vec<u8> = vec![0; packet_length as usize - id_size];
    let bytes_written = bytes.read(&mut packet_data)?;
    if bytes_written != packet_length as usize - id_size {
        return Err(Error::new(ErrorKind::InvalidData, "Packet size did not match actual packet data"));
    }

    Ok(Packet {
        packet_length,
        packet_id,
        data: Bytes::from(packet_data),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_int() {
        let test_bytes: Vec<u8> = vec![0xff, 0xff, 0xff, 0xff, 0x07, 0xf8];

        let mut cursor = Cursor::new(Bytes::from(test_bytes));
        assert_eq!(cursor.read_var_int().unwrap().0, 2147483647);
        assert_eq!(cursor.read_u8().unwrap(), 0xf8);
    }

    #[test]
    fn test_packet_consume() {
        let bytes = Bytes::from(vec![0x03, 0x01, 0xff, 0xff, 0xfd, 0xfd]);

        let mut cursor = Cursor::new(bytes);
        let packet = consume_packet(&mut cursor).unwrap();
        assert_eq!(packet.packet_id, 0x01);
        assert_eq!(packet.packet_length, 0x03);
        assert_eq!(packet.data, Bytes::from(vec![0xff, 0xff]));
        assert_eq!(cursor.position(), 4);
        assert_eq!(cursor.into_inner().to_vec(), vec![0x03, 0x01, 0xff, 0xff, 0xfd, 0xfd]);
    }
}
