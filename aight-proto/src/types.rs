use bytes::BytesMut;
use prost::Message;
include!(concat!(env!("OUT_DIR"), "/aight_proto.msg_types.rs"));

pub enum TypeID {
    Login = 1,
    EchoRequest,
    SendRequest,
}

pub fn create_login(id: String) -> RawTcpMessage {
    to_raw(TypeID::Login as i32, Login { id })
}

pub fn create_echo(body: String) -> RawTcpMessage {
    to_raw(TypeID::EchoRequest as i32, EchoRequest { body })
}

pub fn create_send(to_id: String, body: String) -> RawTcpMessage {
    to_raw(TypeID::SendRequest as i32, MsgSendRequest { to_id, body })
}

fn to_raw<T: Message>(type_id: i32, msg: T) -> RawTcpMessage {
    let mut buffer = BytesMut::with_capacity(msg.encoded_len());
    msg.encode(&mut buffer).expect("unreachable");
    RawTcpMessage {
        type_id,
        body: buffer.to_vec(),
    }
}
