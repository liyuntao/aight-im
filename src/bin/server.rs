use async_std::{
    prelude::*,
    io::BufReader,
    task,
    net::{TcpListener, TcpStream},
};
use std::net::ToSocketAddrs;

use futures::channel::mpsc;
use futures::SinkExt;

use std::sync::Arc;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;


// impl trait: static dispatch, type without name
// It means that function returns some specific type that implements SomeTrait.
async fn server(addr: impl ToSocketAddrs) -> Result<()> {
    let listener = TcpListener::bind(addr).await?;

    let (broker_sender, broker_receiver) = mpsc::unbounded();
    let broker = task::spawn(broker(broker_receiver));

    println!("broker started");
    let mut incoming = listener.incoming();
    // while let xx = xx.await pattern to replace
    // 'async for-loop' which is not supported by rust-lang yet
    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        spawn_and_log_error(client(broker_sender.clone(), stream));
    }
    drop(broker_sender);
//    broker_sender.close(); // TODO: why
    broker.await;
    println!("Server closed!!");
    Ok(())
}

async fn client(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) =>{
            let line = line?;
            if line == "shit" {
                Err("disable server")?
            } else {
                line
            }
        },
    };
    broker.send(Event::NewPeer { name: name.clone(), stream: Arc::clone(&stream)}).await.unwrap();

    while let Some(line) = lines.next().await {
        let line = line?;
        let (dest, msg) = match line.find(':') {
            None => continue,
            Some(idx) => (&line[..idx], line[idx + 1..].trim()),
        };
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.trim().to_string();

        broker.send(Event::Message { from: name.clone(), to: dest, msg, }).await.unwrap();
    }
    Ok(())
}

fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
    where F: Future<Output=Result<()>> + Send + 'static
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("task error: {}", e)
        }
    })
}


type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

async fn client_writer(mut messages: Receiver<String>, stream: Arc<TcpStream>) -> Result<()> {
    let mut stream = &*stream;
    while let Some(msg) = messages.next().await {
        stream.write_all(msg.as_bytes()).await?;
    }
    Ok(())
}

#[derive(Debug)]
enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
}

// actor model
async fn broker(mut events: Receiver<Event>) -> Result<()> {
    let mut writers = Vec::new();

    let mut peers: HashMap<String, Sender<String>> = HashMap::new();

    while let Some(event) = events.next().await {
        match event {
            Event::Message { from, to, msg} => {
                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        peer.send(format!("from {}: {}\n", from, msg)).await?
                    }
                }
            }
            Event::NewPeer { name, stream } => {
                match peers.entry(name) {
                    Entry::Occupied(..) => (),
                    Entry::Vacant(entry) => {
                        let (client_sender, client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);
                        let handle = spawn_and_log_error(client_writer(client_receiver, stream));
                        writers.push(handle);
                    }
                }
            }
        }
    }
    drop(peers); // TODO

    for writer in writers {
        writer.await;
    }

    Ok(())
}

fn main() -> Result<()> {
    let fut = server("127.0.0.1:8080");
    task::block_on(fut)
}
