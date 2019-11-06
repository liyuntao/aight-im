pub mod msg_types {
    use bytes::BytesMut;
    use prost::Message;
    include!(concat!(env!("OUT_DIR"), "/aight_proto.msg_types.rs"));

    pub fn create_login(id: String) -> RawTcpMessage {
        to_raw(1, Login { id })
    }

    pub fn create_echo(body: String) -> RawTcpMessage {
        to_raw(2, EchoRequest { body })
    }

    pub fn create_send(to_id: String, body: String) -> RawTcpMessage {
        to_raw(3, MsgSendRequest { to_id, body })
    }

    fn to_raw<T: Message>(type_id: i32, msg: T) -> RawTcpMessage {
        let mut buffer = BytesMut::with_capacity(msg.encoded_len());
        msg.encode(&mut buffer);
        RawTcpMessage {
            type_id,
            body: buffer.to_vec(),
        }
    }
}
