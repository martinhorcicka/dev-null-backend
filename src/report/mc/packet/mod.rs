pub mod handshake;
pub mod ping;
pub mod status;

use std::{
    io::{self, Read, Write},
    net::TcpStream,
};

use super::bytes::{Bytes, Varint};

#[derive(Debug)]
pub struct Packet {
    pub id: Varint,
    pub data: Bytes,
}

fn leb128_err_to_ioerr(err: leb128::read::Error) -> std::io::Error {
    match err {
        leb128::read::Error::IoError(err) => err,
        leb128::read::Error::Overflow => std::io::Error::new(std::io::ErrorKind::Other, "Overflow"),
    }
}

pub trait SendPacket {
    fn send_packet(self, stream: &mut TcpStream) -> io::Result<usize>
    where
        Self: Sized + Into<Packet>,
    {
        self.into().send(stream)
    }
}

impl Packet {
    pub fn send(self, stream: &mut TcpStream) -> io::Result<usize> {
        let data: Vec<u8> = self.into();
        stream.write(&data)
    }

    pub fn recv(stream: &mut TcpStream) -> io::Result<Packet> {
        let length = leb128::read::signed(stream).map_err(leb128_err_to_ioerr)?;
        let mut buffer = vec![0u8; length as usize];
        stream.read_exact(&mut buffer)?;
        let mut data: Bytes = buffer.into();

        let id = data.get_varint();
        Ok(Packet { id, data })
    }
}

impl From<Packet> for Vec<u8> {
    fn from(packet: Packet) -> Self {
        let mut bytes: Bytes = vec![].into();
        bytes.put_varint(1 + packet.data.len() as Varint);
        bytes.put_varint(packet.id);
        bytes.put_bytebuffer(packet.data);

        bytes.into()
    }
}
