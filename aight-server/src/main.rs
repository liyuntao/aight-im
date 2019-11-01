#![allow(dead_code)]
use std::error::Error;
use tokio::{io::{AsyncWriteExt, BufReader}, net::{TcpListener, TcpStream}, sync::mpsc};
use tokio_io::split;
use tokio_io::split::WriteHalf;
use std::collections::HashMap;
use futures::StreamExt;
use std::collections::hash_map::Entry;
use tokio::io::AsyncBufReadExt;
use std::sync::{Arc};
use tokio::sync::Mutex;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
type Tx<T> = mpsc::UnboundedSender<T>;
type Rx<T> = mpsc::UnboundedReceiver<T>;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080".to_string();

    let mut listener = TcpListener::bind(&addr).await?;
    println!("Aight-Server is listing on 8080");

    let broker = Arc::new(Mutex::new(Broker::new()));

    loop { // accept loop
        let (socket, _) = listener.accept().await?;
        let broker = Arc::clone(&broker);
        tokio::spawn(accept_handle_loop(broker,socket));
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

    async fn on_event(&mut self, id: &str, event: Event) {
        match event {
            Event::NewPeer { stream: mut write_half } => {
                println!("user login; id: {}", id);
                match self.peers.entry(id.to_string()) { // TODO duplicated copy here?
                    Entry::Occupied(..) => {
                        println!("!!another client incoming with same id; id: {}; shutdown now", id);
                        // send help msg to client-side
                        write_half.shutdown().await;
                    },
                    Entry::Vacant(entry) => {
                        // hold session and session_status in hashmap
                        entry.insert(Peer::new(write_half, id.to_string() ) );
                    }
                }
            }
            Event::Message { to, msg } => {
                if let Some(peer) = self.peers.get_mut(&to) {
                    let content = format!("msg from {}: {}\n", id, msg);
                    peer.socket.write_all(content.as_bytes()).await.unwrap();
                }
            }
            Event::Echo { msg } => {
                if let Some(peer) = self.peers.get_mut(id) {
                    peer.socket.write_all(msg.as_bytes()).await.unwrap();
                }
            }
            Event::Exit => {
                println!("client-side exited: id={};", id);
                self.remove_peer(id);
            }
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
        Peer { socket, status: PeerStatus::Authorized(id) }
    }
}

async fn accept_handle_loop(broker: Arc<Mutex<Broker>>,
                            socket: TcpStream,
) {
    // 1. read from socket & parsing
    let (read_half, write_half) = split::split(socket);
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();

    // 2. hold connection/session
    let id = match lines.next().await {
        None => {
            // no id coming, peer disconnected immediately
            return
        },
        Some(line) => line.unwrap(),
    };

    {
        let mut broker = broker.lock().await;
        broker.on_event(&id, Event::NewPeer { stream: write_half }).await;
    }

    // 3. transfer parsed events to broker
    while let Some(raw_line) = lines.next().await {
        let line = raw_line.unwrap();
        println!("raw_line incoming: id={}, raw={}", id, line);
        if let Some(event) = parse_raw(line) {
            let mut broker = broker.lock().await;
            broker.on_event(&id, event).await;
        }
    }

    // if logic goes here, means this connection is closed.
    {
        println!("peer closed!! id={}", id);
        let mut broker = broker.lock().await;
        broker.remove_peer(&id).await;
    }
}

fn parse_raw(line: String) -> Option<Event> {
    if line.starts_with(":to") {
        let idx = line.find(' ')?;
        let left = line[idx + 1 ..].trim();
        let idx = left.find(' ')?;
        let (target_id, content) = (&left[..idx], left[idx + 1 ..].trim());
        Some( Event::Message {
            to: target_id.to_string(),
            msg: content.to_string()
        })
    } else if line.starts_with(":echo") {
        let idx = line.find(' ')?;
        let content = line[idx + 1 ..].trim();
        Some(Event::Echo { msg: content.to_string() })
    } else if line.starts_with(":exit") {
        Some(Event::Exit)
    } else {
        None
    }
}

#[derive(Debug)]
enum Event {
    NewPeer {
        stream: WriteHalf<TcpStream>,
    },
    Message {
        to: String,
        msg: String,
    },
    Echo {
        msg: String,
    },
    Exit,
}
