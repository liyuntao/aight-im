use std::error::Error;
use tokio;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080".to_string();

    let mut listener = TcpListener::bind(&addr).await?;
    println!("Listing on 8080");

    loop {
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0; 4096];
            loop {
                let n = socket
                    .read(&mut buf)
                    .await
                    .expect("failed to read data from socket");

                if n == 0 {
                    return;
                }
                socket
                    .write_all(&buf[0..n])
                    .await
                    .expect("failed to write data to socket");
            }
        });
    }
}
