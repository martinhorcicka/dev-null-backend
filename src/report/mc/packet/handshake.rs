use bytes::BufMut;

use crate::report::mc::bytes::{Bytes, Varint};

use super::{Packet, SendPacket};

pub struct Handshake {
    pub protocol_version: Varint,
    pub server_address: String,
    pub server_port: u16,
    pub next_state: NextState,
}

pub enum NextState {
    Status,
    _Login,
}

impl Default for Handshake {
    fn default() -> Self {
        Self {
            protocol_version: 760,
            server_address: "127.0.0.1".to_string(),
            server_port: 25565,
            next_state: NextState::Status,
        }
    }
}
impl SendPacket for Handshake {}

impl From<Handshake> for Bytes {
    fn from(data: Handshake) -> Self {
        let mut handshake_data: Bytes = vec![].into();
        handshake_data.put_varint(data.protocol_version);
        handshake_data.put_string(&data.server_address);
        handshake_data.put_u16(data.server_port);
        handshake_data.put_varint(match data.next_state {
            NextState::Status => 0x01,
            NextState::_Login => 0x00,
        });
        handshake_data
    }
}

impl From<Handshake> for Packet {
    fn from(data: Handshake) -> Self {
        Self {
            id: 0x00,
            data: data.into(),
        }
    }
}
