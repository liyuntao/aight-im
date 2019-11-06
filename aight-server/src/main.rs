#![allow(dead_code)]
use futures::StreamExt;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tokio_io::split;
use tokio_io::split::WriteHalf;
use tokio::codec::FramedRead;
use aight_proto::codec;
use aight_proto::types::*;

#[macro_use]
extern crate log;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
type Tx<T> = mpsc::UnboundedSender<T>;
type Rx<T> = mpsc::UnboundedReceiver<T>;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let addr = "127.0.0.1:8080".to_string();

    let mut listener = TcpListener::bind(&addr).await?;
    info!("Aight-Server is listing on 8080");

    let broker = Arc::new(Mutex::new(Broker::new()));

    loop {
        // accept loop
        let (socket, _) = listener.accept().await?;
        let broker = Arc::clone(&broker);
        tokio::spawn(accept_handle_loop(broker, socket));
    }
}

struct Broker {
    peers: HashMap<String, Peer>,
}

impl Broker {
    fn new() -> Self {
        Broker {
            peers: HashMap::new(),
        }
    }

    async fn on_event(&mut self, id: &str, event: ServerEvent) {
        match event {
            ServerEvent::NewPeer {
                stream: mut write_half,
            } => {
                debug!("user login. id={}", id);
                match self.peers.entry(id.to_string()) {
                    // TODO duplicated copy here?
                    Entry::Occupied(..) => {
                        debug!(
                            "another client connected with same id! id={}; refuse connection",
                            id
                        );
                        write_half
                            .write_all(
                                "another client with same id is online. connection refused."
                                    .as_bytes(),
                            )
                            .await;
                        // send help msg to client-side
                        write_half.shutdown().await;
                    }
                    Entry::Vacant(entry) => {
                        // hold session and session_status in hashmap
                        entry.insert(Peer::new(write_half, id.to_string()));
                    }
                }
            }
            ServerEvent::Message { to, msg } => {
                if let Some(peer) = self.peers.get_mut(&to) {
                    let content = format!("msg from {}: {}\n", id, msg);
                    peer.socket.write_all(content.as_bytes()).await.unwrap();
                }
            }
            ServerEvent::Echo { msg } => {
                if let Some(peer) = self.peers.get_mut(id) {
                    peer.socket.write_all(msg.as_bytes()).await.unwrap();
                }
            }
            ServerEvent::Exit => {
                self.remove_peer(id).await;
            }
            _ => {},
        }
    }

    async fn remove_peer(&mut self, id: &str) {
        if let Some(peer) = self.peers.get_mut(id) {
            peer.socket.shutdown().await;
            self.peers.remove(id);
        }
    }
}

enum PeerStatus {
    Unauthorized,
    Authorized(String),
}
struct Peer {
    socket: WriteHalf<TcpStream>,
    status: PeerStatus,
}

impl Peer {
    fn new(socket: WriteHalf<TcpStream>, id: String) -> Self {
        Peer {
            socket,
            status: PeerStatus::Authorized(id),
        }
    }
}

async fn accept_handle_loop(broker: Arc<Mutex<Broker>>, socket: TcpStream) {
    // 1. read from socket & parsing
    let (read_half, write_half) = split::split(socket);
    let mut reader = FramedRead::new(read_half, codec::ProtobufFrameCodec::new());

    let first_msg = reader.next().await;

    // 2. hold connection/session
    let id = match first_msg {
        None => {
            // no id coming, peer disconnected immediately
            return;
        }
        Some(msg) => {
            let first_msg = parse_raw_to_event(msg.unwrap());
            if let ServerEvent::Login { id }  = first_msg {
                id
            } else {
                return;
            }
        },
    };

    {
        let mut broker = broker.lock().await;
        broker
            .on_event(&id, ServerEvent::NewPeer { stream: write_half })
            .await;
    }

    // 3. transfer parsed events to broker
    while let Some(raw_msg) = reader.next().await {
        match raw_msg {
            Ok(raw_msg) => {
                let event = parse_raw_to_event(raw_msg);
                let mut broker = broker.lock().await;
                broker.on_event(&id, event).await;
            },
            Err(e) => {
                error!("error while decode bytes: {}", e);
            }
        }
    }

    // if logic goes here, means this connection is closed.
    {
        debug!("peer closed!! id={}", id);
        let mut broker = broker.lock().await;
        broker.remove_peer(&id).await;
    }
}

fn parse_raw_to_event(raw: RawTcpMessage) -> ServerEvent {
    let type_id = raw.type_id;
    let bytes = raw.body;

    if type_id == 1 {
        let msg: Login = parse_raw_msg(bytes).unwrap();
        ServerEvent::Login { id: msg.id }
    } else if type_id == 2 {
        let msg: EchoRequest = parse_raw_msg(bytes).unwrap();
        ServerEvent::Echo { msg: msg.body }
    } else if type_id == 3 {
        let msg: MsgSendRequest = parse_raw_msg(bytes).unwrap();
        ServerEvent::Message { to: msg.to_id, msg: msg.body }
    } else {
        ServerEvent::Unknown
    }
}

#[derive(Debug)]
enum ServerEvent {
    NewPeer { stream: WriteHalf<TcpStream> },
    Login { id: String },
    Message { to: String, msg: String },
    Echo { msg: String },
    Unknown,
    Exit,
}
