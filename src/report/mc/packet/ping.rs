use bytes::{Buf, BufMut};
use serde::Serialize;

use crate::report::mc::bytes::Bytes;

use super::{Packet, SendPacket};

pub struct PingRequest {
    pub payload: i64,
}
impl SendPacket for PingRequest {}

impl From<PingRequest> for Packet {
    fn from(ping: PingRequest) -> Self {
        let mut data: Bytes = vec![].into();
        data.put_i64(ping.payload);

        Packet { id: 0x01, data }
    }
}

#[derive(Serialize)]
pub struct PingResponse {
    payload: i64,
}

impl TryFrom<Packet> for PingResponse {
    type Error = String;

    fn try_from(mut packet: Packet) -> Result<Self, Self::Error> {
        match packet.id {
            0x01 => {
                let payload = packet.data.get_i64();
                Ok(Self { payload })
            }
            _ => Err("wrong packet id".to_string()),
        }
    }
}
