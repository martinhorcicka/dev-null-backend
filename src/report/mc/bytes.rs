use bytes::{Buf, BufMut};

// use super::status_packet::ResourceLocation;
pub type Varint = i64;

#[derive(Debug)]
pub struct Bytes(bytes::BytesMut);
unsafe impl BufMut for Bytes {
    fn remaining_mut(&self) -> usize {
        self.0.remaining_mut()
    }

    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.0.advance_mut(cnt)
    }

    fn chunk_mut(&mut self) -> &mut bytes::buf::UninitSlice {
        self.0.chunk_mut()
    }
}

impl Buf for Bytes {
    fn remaining(&self) -> usize {
        self.0.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.0.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.0.advance(cnt)
    }
}

impl std::io::Write for Bytes {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.put_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Bytes {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Bytes {
    pub fn get_bool(&mut self) -> bool {
        self.get_u8() > 0
    }

    pub fn get_varint(&mut self) -> Varint {
        let mut bytes = vec![];
        let mut byte = self.get_u8();
        bytes.push(byte);
        while byte & 0x80 != 0 {
            byte = self.get_u8();
            bytes.push(byte);
        }

        leb128::read::signed(&mut &bytes[..]).expect("should be varint")
    }

    pub fn get_string(&mut self) -> String {
        let string_length = self.get_varint();
        let mut string_bytes = vec![0u8; string_length as usize];
        self.copy_to_slice(&mut string_bytes[..]);

        String::from_utf8_lossy(string_bytes.as_slice()).into()
    }
}

impl Bytes {
    pub fn put_varint(&mut self, value: Varint) {
        leb128::write::signed(self, value).expect("write i64 as a varint to ByteBuffer");
    }

    pub fn put_vec(&mut self, vec: &[u8]) {
        self.put_slice(vec);
    }

    pub fn put_bytebuffer(&mut self, buf: Self) {
        let slice: bytes::Bytes = buf.0.into();
        self.put_vec(&slice);
    }

    pub fn put_string(&mut self, string: &String) {
        let string_length = string.len();
        let string_bytes = string.as_bytes();

        self.put_varint(string_length as Varint);
        self.put_slice(string_bytes);
    }
}

impl<'a> From<&'a [u8]> for Bytes {
    fn from(value: &[u8]) -> Self {
        Bytes(value.into())
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(value: Vec<u8>) -> Self {
        value.as_slice().into()
    }
}

impl From<Bytes> for Vec<u8> {
    fn from(value: Bytes) -> Self {
        value.0.into()
    }
}
