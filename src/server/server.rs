#![allow(dead_code)]

use std::error::Error;
use tokio::{io::{AsyncWriteExt, BufReader}, net::{TcpListener, TcpStream}, sync::mpsc};
use tokio_io::split;
use tokio_io::split::WriteHalf;
use std::collections::HashMap;
use futures::{StreamExt, SinkExt};
use std::collections::hash_map::Entry;
use tokio::io::AsyncBufReadExt;
use std::sync::{Arc};
use tokio::sync::Mutex;
use futures::async_await::assert_fused_stream;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;
type Tx<T> = mpsc::UnboundedSender<T>;
type Rx<T> = mpsc::UnboundedReceiver<T>;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080".to_string();

    let mut listener = TcpListener::bind(&addr).await?;
    println!("Aight-Server is listing on 8080");

    let broker = Arc::new(Mutex::new(Broker::new()));

//    let (broker_sender, broker_receiver) = mpsc::unbounded_channel();
//    let _broker_task = tokio::spawn(broker_handle(broker_receiver));

    loop { // accept loop
        let (socket, _) = listener.accept().await?;
        let broker = Arc::clone(&broker);
        tokio::spawn(accept_handle_loop(broker,broker_sender.clone(), socket));
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

    async fn on_event(&mut self, event: Event) {
        match event {
            Event::NewPeer { id, stream: mut write_half } => {
                println!("user login; id: {}", id);

                match peers.entry(id) {
                    Entry::Occupied(..) => {
                        println!("!!another client incoming with same id; id: {}; shutdown now", id);
                        // send help msg to client-side
                        write_half.shutdown().await;
                    },
                    Entry::Vacant(entry) => {
                        // hold session and session_status in hashmap
                        entry.insert(Peer::new(write_half) );
                    }
                }
            }
            Event::Message { id, to, msg } => {
                if let Some(peer) = self.peers.get_mut(&to) {
                    let content = format!("msg from {}: {}\n", id, msg);
                    peer.socket.write_all(content.as_bytes()).await.unwrap();
                }
            }
            Event::Exit { id} => {
                println!("client-side exited: id={};", id);
                self.remove_peer(id);
            }
        }
    }

    async fn remove_peer(&mut self, id: String) {
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.socket.shutdown().await;
            self.peers.remove(&id);
        }
    }
}

enum PeerStatus {
    Unauthorized,
    Authorized(String), // id
}
struct Peer {
    socket: WriteHalf<TcpStream>,
    status: PeerStatus,
}

impl Peer {
    fn new(socket: WriteHalf<TcpStream>) -> Self {
        Peer { socket, status: PeerStatus::Unauthorized }
    }
}

async fn accept_handle_loop(broker: Arc<Mutex<Broker>>,
                            mut broker_tx: Tx<Event>,
                            socket: TcpStream,
) {
    // 1. read from socket & parsing
    let (read_half, write_half) = split::split(socket);
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();

    // 2. hold connection/session
    let id = match lines.next().await {
        None => return,
        Some(line) => line.unwrap(),
    };
    broker_tx.send(Event::NewPeer {
        id,
        stream: write_half,
    },).await.unwrap();

    // 3. transfer parsed events to broker
    while let Some(raw_line) = lines.next().await {
        let line = raw_line.unwrap();
        if let Some(event) = parse_raw(line) {
            broker_tx.try_send(event).unwrap();
        }
    }
}

fn parse_raw(line: String) -> Option<Event> {
    println!("debug: raw line incoming: {}", line);

    match line.find('|') {
        None => {
            None
        },
        Some(idx) => {
            let (source_id, left) = (&line[..idx], line[idx + 1 ..].trim());
            if left.starts_with(":to") {
                let idx = left.find(' ')?;
                let left = left[idx + 1 ..].trim();
                let idx = left.find(' ')?;
                let (target_id, content) = (&left[..idx], left[idx + 1 ..].trim());
                Some( Event::Message {id: source_id.to_string(),
                    to: target_id.to_string(),
                    msg: content.to_string() })
            } else if left.starts_with(":echo") {
                let idx = left.find(' ')?;
                let content = left[idx + 1 ..].trim();
                Some( Event::Message {id: source_id.to_string(),
                    to: source_id.to_string(),
                    msg: content.to_string() })
            } else if left.starts_with(":exit") {
                Some(Event::Exit {id: source_id.to_string() })
            } else {
                None
            }
        },
    }
}

#[derive(Debug)]
enum Event {
    NewPeer {
        id: String,
        stream: WriteHalf<TcpStream>,
    },
    Message {
        id: String,
        to: String,
        msg: String,
    },
    Exit {
        id: String,
    },
}

//async fn broker_handle(mut events: Rx<Event>) {
//    let mut peers: HashMap<String, Tx<String>> = HashMap::new();
//
//    while let Some(event) = events.next().await {
//        match event {
//            Event::NewPeer { id, stream: write_half } => {
//                println!("user login; id: {}", id);
//
//                match peers.entry(id) {
//                    Entry::Occupied(..) => (),
//                    Entry::Vacant(entry) => {
//                        let (peer_tx, peer_rx) = mpsc::unbounded_channel();
//                        entry.insert(peer_tx);
//                        // TODO error handling
//                        tokio::spawn(peer_handle(peer_rx, write_half));
//                    }
//                }
//            }
//            Event::Message { id, to, msg } => {
//                if let Some(peer) = peers.get_mut(&to) {
//                    let content = format!("msg from {}: {}\n", id, msg);
//                    peer.send(content).await.unwrap();
//                }
//            }
//            Event::Exit { id} => {
//                println!("client disconnected; id: {}", id);
//                // TODO close chain: on-exit-event -> channel -> socket??
//            }
//        }
//    }
//}

async fn peer_handle(mut msg_out: Rx<String>, mut write_half: WriteHalf<TcpStream>) {
    while let Some(msg) = msg_out.next().await {
        println!("debug: msg to sb: {}", msg);
        write_half.write_all(msg.as_bytes()).await.unwrap();
    }
}
