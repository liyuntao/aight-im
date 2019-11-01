use futures::executor::LocalPool;
use std::env;
use std::error::Error;
use tokio::{
    codec::{FramedRead, FramedWrite},
    io,
    prelude::*,
    sync::mpsc,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    LocalPool::new().run_until(run_client())
}

async fn run_client() -> Result<()> {
    let id = env::args()
        .nth(1)
        .expect("must provide an user_id, e.g. client tom");
    println!("Aight-Client is started with id: {}", id);
    let stdin = async_stdin();
    let stdout = FramedWrite::new(io::stdout(), codec::Bytes);
    conn::connect(id, &"127.0.0.1:8080".parse().unwrap(), stdin, stdout).await?;
    Ok(())
}

fn async_stdin() -> impl Stream<Item = std::result::Result<Vec<u8>, io::Error>> + Unpin {
    let mut stdin = FramedRead::new(io::stdin(), codec::Bytes);
    let (mut tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        tx.send_all(&mut stdin).await.unwrap();
    });
    rx
}

mod conn {
    use super::codec;
    use colored::*;
    use futures::{future, Sink, SinkExt, Stream, StreamExt};
    use std::net::SocketAddr;
    use std::{error::Error, io};
    use tokio::{
        codec::{FramedRead, FramedWrite},
        io::AsyncWriteExt,
        net::TcpStream,
    };

    pub async fn connect(
        id: String,
        addr: &SocketAddr,
        stdin: impl Stream<Item = Result<Vec<u8>, io::Error>> + Unpin,
        mut stdout: impl Sink<Vec<u8>, Error = io::Error> + Unpin,
    ) -> Result<(), Box<dyn Error>> {
        let mut stream = TcpStream::connect(addr).await?;
        let (r, mut w) = stream.split();

        // on connected
        w.write_all(format!("{}\n", id).as_bytes()).await?;
        w.flush().await?;

        let sink = FramedWrite::new(w, codec::Bytes);
        let mut stream = FramedRead::new(r, codec::Bytes).filter_map(|i| match i {
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

        match future::join(stdin.forward(sink), stdout.send_all(&mut stream)).await {
            (Err(e), _) | (_, Err(e)) => Err(e.into()),
            _ => Ok(()),
        }
    }
}

mod codec {
    use bytes::{BufMut, BytesMut};
    use std::io;
    use tokio::codec::{Decoder, Encoder};

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
}
