use futures::executor::LocalPool;
use std::{env, error::Error};
use tokio::{
    codec::{FramedRead, FramedWrite, LinesCodec},
    io,
    net::TcpStream,
    prelude::*,
    sync::mpsc,
};

use aight_proto::msg_types::*;
use colored::*;
use futures::{future, Sink, SinkExt, Stream, StreamExt};
use prost::Message;
use std::io::ErrorKind;
use std::net::SocketAddr;
use tokio::codec::LinesCodecError;

mod codec;

#[tokio::main]
async fn main() {
    LocalPool::new().run_until(run_client());
}

async fn run_client() {
    let id = env::args()
        .nth(1)
        .expect("must provide an user_id, e.g. client tom");
    println!("Aight-Client is started with id: {}", id);
    let stdin = async_stdin();
    let stdout = FramedWrite::new(io::stdout(), codec::Bytes);
    connect(id, &"127.0.0.1:8080".parse().unwrap(), stdin, stdout).await;
}

fn async_stdin() -> impl Stream<Item = std::result::Result<String, LinesCodecError>> + Unpin {
    let mut stdin = FramedRead::new(io::stdin(), LinesCodec::new());
    let (mut tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        tx.send_all(&mut stdin).await.unwrap();
    });
    rx
}

fn parse_line_to_msg(line: String) -> Option<RawTcpMessage> {
    if line.starts_with(":to") {
        let idx = line.find(' ')?;
        let left = line[idx + 1..].trim();
        let idx = left.find(' ')?;
        let (target_id, content) = (&left[..idx], left[idx + 1..].trim());
        Some(create_send(target_id.to_string(), content.to_string()))
    } else if line.starts_with(":echo") {
        let idx = line.find(' ')?;
        let content = line[idx + 1..].trim();
        Some(create_echo(content.to_string()))
    } else {
        None
    }
}

async fn transform(
    res: Result<String, LinesCodecError>,
) -> Option<Result<RawTcpMessage, io::Error>> {
    res.map(parse_line_to_msg)
        .map_err(|e| match e {
            LinesCodecError::MaxLineLengthExceeded => io::Error::from(ErrorKind::Other),
            LinesCodecError::Io(e) => e,
        })
        .transpose()
}

pub async fn connect(
    id: String,
    addr: &SocketAddr,
    stdin: impl Stream<Item = std::result::Result<String, LinesCodecError>> + Unpin,
    mut stdout: impl Sink<Vec<u8>, Error = io::Error> + Unpin,
) -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect(addr).await?;
    let (socket_r, mut socket_w) = stream.split();

    let mut sink = FramedWrite::new(socket_w, codec::ProtobufFrameCodec);
    // on connected, send LoginReq
    sink.send(create_login(id)).await;

    let mut stream = FramedRead::new(socket_r, codec::Bytes).filter_map(|i| match i {
        Ok(i) => {
            let colored_str = String::from_utf8(i).unwrap().green();
            let out_bytes = format!("> {}\n", colored_str).into_bytes();
            future::ready(Some(out_bytes))
        }
        Err(e) => {
            println!("failed to read from socket; error={}", e);
            future::ready(None)
        }
    });

    match future::join(
        stdin.filter_map(transform).forward(sink),
        stdout.send_all(&mut stream),
    )
    .await
    {
        (Err(e), _) | (_, Err(e)) => Err(e.into()),
        _ => Ok(()),
    }
}
