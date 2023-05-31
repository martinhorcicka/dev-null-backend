use std::collections::HashMap;

use bytes::Buf;
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    report::mc::bytes::{Bytes, Varint},
};

use super::{Packet, SendPacket};

#[derive(Default)]
pub struct StatusRequest;
impl SendPacket for StatusRequest {}

impl From<StatusRequest> for Bytes {
    fn from(_: StatusRequest) -> Self {
        vec![].into()
    }
}

impl From<StatusRequest> for Packet {
    fn from(data: StatusRequest) -> Self {
        Self {
            id: 0x00,
            data: data.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResponse {
    pub previews_chat: bool,
    pub enforces_secure_chat: bool,
    pub description: Description,
    pub players: Players,
    pub version: Version,
    pub favicon: String,
    pub forge_data: ForgeData,
    pub prevents_chat_reports: bool,
}

#[derive(Deserialize, Serialize)]
pub struct Description {
    text: String,
}

#[derive(Deserialize, Serialize)]
pub struct Players {
    max: i32,
    online: i32,
}
#[derive(Deserialize, Serialize)]
pub struct Version {
    name: String,
    protocol: Varint,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeData {
    pub fml_network_version: i32,
    pub d: ForgeDataD,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ForgeDataD {
    pub truncated: bool,
    pub mods_size: u16,
    pub mods: HashMap<String, String>,
    pub channels: HashMap<ResourceLocation, (String, bool)>,
}

#[derive(PartialEq, Eq, Hash)]
pub struct ResourceLocation {
    pub mod_id: String,
    pub channel_name: String,
}

impl Serialize for ResourceLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let format = format!("{}:{}", self.channel_name, self.mod_id);
        serializer.serialize_str(&format)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnparsedStatusResponse {
    previews_chat: bool,
    enforces_secure_chat: bool,
    description: Description,
    players: Players,
    version: Version,
    favicon: String,
    forge_data: UnparsedForgeData,
    prevents_chat_reports: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnparsedForgeData {
    fml_network_version: i32,
    d: String,
}

impl From<UnparsedStatusResponse> for StatusResponse {
    fn from(p: UnparsedStatusResponse) -> Self {
        Self {
            previews_chat: p.previews_chat,
            enforces_secure_chat: p.enforces_secure_chat,
            description: p.description,
            players: p.players,
            version: p.version,
            favicon: p.favicon,
            forge_data: p.forge_data.into(),
            prevents_chat_reports: p.prevents_chat_reports,
        }
    }
}

impl From<UnparsedForgeData> for ForgeData {
    fn from(ufd: UnparsedForgeData) -> Self {
        Self {
            fml_network_version: ufd.fml_network_version,
            d: decode_forge_data_d(ufd.d),
        }
    }
}

impl TryFrom<Packet> for StatusResponse {
    type Error = Error;

    fn try_from(mut packet: Packet) -> Result<Self, Self::Error> {
        if packet.id != 0x00 {
            return Err(Error::Generic("wrong packet id".to_string()));
        }

        let unparsed_status_response: UnparsedStatusResponse =
            serde_json::from_str(&packet.data.get_string())?;

        Ok(unparsed_status_response.into())
    }
}

const VERSION_FLAG_IGNORESERVERONLY: i64 = 0b1;
const IGNORESERVERONLY: &str = "not specified";
fn decode_forge_data_d(d: String) -> ForgeDataD {
    let decoded_data = decode_optimized(d);
    let mut d: Bytes = decoded_data.into();
    let truncated = d.get_bool();
    let mods_size = d.get_u16();

    let mut mods = HashMap::new();
    let mut channels = HashMap::new();

    for _ in 0..mods_size {
        let channel_size_and_version_flag = d.get_varint();
        let channel_size = channel_size_and_version_flag >> 1;
        let is_ignore_server_only =
            (channel_size_and_version_flag & VERSION_FLAG_IGNORESERVERONLY) != 0;

        let mod_id = d.get_string();
        let mod_version = if is_ignore_server_only {
            IGNORESERVERONLY.to_string()
        } else {
            d.get_string()
        };

        for _ in 0..channel_size {
            let channel_name = d.get_string();
            let channel_version = d.get_string();
            let required_on_client = d.get_bool();
            let res_loc = ResourceLocation {
                mod_id: mod_id.clone(),
                channel_name,
            };
            let pair = (channel_version, required_on_client);
            channels.insert(res_loc, pair);
        }

        mods.insert(mod_id, mod_version);
    }

    let non_mod_channel_count = d.get_varint();
    for _ in 0..non_mod_channel_count {
        let res_loc = d.get_string();
        let res_loc_data: Vec<&str> = res_loc.split(':').collect();
        let channel_name = ResourceLocation {
            channel_name: res_loc_data[0].to_string(),
            mod_id: res_loc_data[1].to_string(),
        };

        let channel_version = d.get_string();
        let required_on_client = d.get_bool();

        channels.insert(channel_name, (channel_version, required_on_client));
    }
    ForgeDataD {
        truncated,
        mods_size,
        mods,
        channels,
    }
}
fn decode_optimized(s: String) -> Vec<u8> {
    let data: Vec<u16> = s.encode_utf16().collect();
    let mut current = &data[..];

    let size0 = (current[0] & 0x7FFF) as u32;
    let size1 = (current[1] & 0x7FFF) as u32;
    let size = size0 | (size1 << 15);
    let size = size as usize;

    current = &current[2..];

    let mut buf = Vec::with_capacity(size);

    let mut buffer: u32 = 0;
    let mut bits_in_buffer: u8 = 0;

    while !current.is_empty() {
        while bits_in_buffer >= 8 {
            buf.push((buffer & 0xFF) as u8);
            buffer >>= 8;
            bits_in_buffer -= 8;
        }

        let c = current[0];
        current = &current[1..];
        buffer |= (c as u32) << bits_in_buffer;
        bits_in_buffer += 15;
    }

    while buf.len() < size {
        buf.push(buffer as u8);
        buffer >>= 8;
        if bits_in_buffer >= 8 {
            bits_in_buffer -= 8;
        } else {
            bits_in_buffer -= bits_in_buffer;
        }
    }

    buf
}
