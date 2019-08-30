//use futures::channel::mpsc;
//use futures::SinkExt;
//
//use std::sync::Arc;
//use async_std::net::TcpStream;
//
//type Sender<T> = mpsc::UnboundedSender<T>;
//type Receiver<T> = mpsc::UnboundedReceiver<T>;
//
//async fn client_writer(mut messages: Receiver<String>, stream: Arc<TcpStream>) -> Result<()> {
//    let mut stream = &*stream;
//    while let Some(msg) = messages.next().await {
//        stream.write_all(msg.as_bytes()).await?;
//    }
//    Ok(())
//}


fn main() {

}