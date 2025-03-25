/*use std::io;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use log::{info, error};
use env_logger;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    info!("Starting client...");
    println!("Starting client...");

    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    info!("Connected to server at 127.0.0.1:8080");
    println!("Connected to server at 127.0.0.1:8080");

    stream.write_all(b"Hello, world!\n").await?;
    info!("Sent message to server");
    println!("Sent message to server");

    let mut buf = vec![0; 1024];
    let n = stream.read(&mut buf).await?;
    info!("Received message from server");
    println!("Received: {}", String::from_utf8_lossy(&buf[..n]));

    Ok(())
}*/