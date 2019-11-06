use crate::types::RawTcpMessage;
use bytes::{BufMut, BytesMut};
use prost::Message;
use std::io;
use tokio_codec::{Decoder, Encoder};

// A codec that splits the received ByteBufs dynamically by the value of
// the Google Protocol Buffers Base 128 Varints integer length field in the message.
// For example:
// BEFORE DECODE (302 bytes)       AFTER DECODE (300 bytes)
// +--------+---------------+      +---------------+
// | Length | Protobuf Data |----->| Protobuf Data |
// | 0xAC02 |  (300 bytes)  |      |  (300 bytes)  |
// +--------+---------------+      +---------------+
#[derive(Debug, Clone)]
pub struct ProtobufFrameCodec;

impl ProtobufFrameCodec {
    pub fn new() -> Self {
        Self {}
    }
}

impl Encoder for ProtobufFrameCodec {
    type Item = RawTcpMessage;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, buf: &mut BytesMut) -> Result<(), Self::Error> {
        let len = item.encoded_len();

        if buf.remaining_mut() < len {
            buf.reserve(len);
        }

        item.encode_length_delimited(buf)
            .map_err(|_| unreachable!("Message only errors if not enough space"))
    }
}

impl Decoder for ProtobufFrameCodec {
    type Item = RawTcpMessage;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Message::decode_length_delimited(buf.take())
            .map(Option::Some)
            .map_err(|err| err.into())
    }
}

pub struct Bytes;

impl Decoder for Bytes {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Vec<u8>>> {
        if buf.len() > 0 {
            let len = buf.len();
            Ok(Some(buf.split_to(len).into_iter().collect()))
        } else {
            Ok(None)
        }
    }
}

impl Encoder for Bytes {
    type Item = Vec<u8>;
    type Error = io::Error;

    fn encode(&mut self, data: Vec<u8>, buf: &mut BytesMut) -> io::Result<()> {
        buf.put(&data[..]);
        Ok(())
    }
}
