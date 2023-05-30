use std::{
    net::{IpAddr, Ipv4Addr, UdpSocket},
    time::Duration,
};

use axum::Json;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::Serialize;

mod space_engineers;

#[derive(Serialize)]
struct A2sInfo {
    header: u8,
    protocol: u8,
    name: String,
    map: String,
    folder: String,
    game: String,
    id: i16,
    players: u8,
    max_players: u8,
    bots: u8,
    server_type: u8,
    environment: u8,
    visibility: u8,
    vac: u8,
    version: String,
    edf: u8,
}

impl From<Bytes> for A2sInfo {
    fn from(mut bytes: Bytes) -> Self {
        let _minus_one = bytes.get_i32();
        let header = bytes.get_u8();
        let protocol = bytes.get_u8();
        let name = bytes.get_string();
        let map = bytes.get_string();
        let folder = bytes.get_string();
        let game = bytes.get_string();
        let id = bytes.get_i16();
        let players = bytes.get_u8();
        let max_players = bytes.get_u8();
        let bots = bytes.get_u8();
        let server_type = bytes.get_u8();
        let environment = bytes.get_u8();
        let visibility = bytes.get_u8();
        let vac = bytes.get_u8();
        let version = bytes.get_string();
        let edf = bytes.get_u8();
        Self {
            header,
            protocol,
            name,
            map,
            folder,
            game,
            id,
            players,
            max_players,
            bots,
            server_type,
            environment,
            visibility,
            vac,
            version,
            edf,
        }
    }
}

fn a2s_info(ip_addr: IpAddr, port: u16) -> Json<A2sInfo> {
    let mut bytes = BytesMut::new();
    bytes.put_i32(-1);
    bytes.put_u8(0x54);
    bytes.put_slice("Source Engine Query".as_bytes());
    bytes.put_u8(0x00);

    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).expect("udp socket bind");
    socket
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("setting socket read timeout failed");
    socket
        .set_write_timeout(Some(Duration::from_secs(5)))
        .expect("setting socket write timeout failed");

    socket
        .connect(format!("{ip_addr}:{port}"))
        .expect("udp socket connect");

    socket.send(&bytes).expect("udb socket send");

    let mut buf = vec![0u8; 1401];
    socket.recv(&mut buf).expect("udp socket receive");
    let bytes: Bytes = buf.into();
    Json(bytes.into())
}

trait GetString {
    fn get_string(&mut self) -> String;
}

impl GetString for Bytes {
    fn get_string(&mut self) -> String {
        let non_zero_byte = |b| {
            if b == 0 {
                None
            } else {
                Some(b)
            }
        };

        let mut bytes = vec![];
        while let Some(b) = non_zero_byte(self.get_u8()) {
            bytes.push(b);
        }

        String::from_utf8(bytes).expect("reading string")
    }
}
