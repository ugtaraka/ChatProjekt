mod client;

use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use log::{info, error};
use serde::Deserialize;
use std::fs;

#[derive(Deserialize)]
struct Config {
    address: String,
    port: u16,
}

#[tokio::main]
async fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    info!("Starting server...");
    println!("Starting server...");

    let config: Config = toml::from_str(&fs::read_to_string("config.toml").expect("Failed to read config file"))
        .expect("Failed to parse config file");

    let addr = format!("{}:{}", config.address, config.port);
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind to address");
    info!("Server listening on {}", addr);
    println!("Listening on {}", addr);

    loop {
        let (mut socket, _) = listener.accept().await.expect("Failed to accept connection");
        info!("Accepted connection");
        println!("Connection received from {}", socket.peer_addr().unwrap());

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => {
                        info!("Connection closed");
                        println!("Connection closed");
                        return;
                    },
                    Ok(n) => n,
                    Err(e) => {
                        error!("Failed to read from socket; err = {:?}", e);
                        println!("Failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                if socket.write_all(&buf[0..n]).await.is_err() {
                    error!("Failed to write to socket");
                    println!("Failed to write to socket");
                    return;
                }
            }
        });
    }
}