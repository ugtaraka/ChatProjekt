use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use warp::Filter;
use log::{info, error};
use serde::Deserialize;
use std::fs;
use std::net::SocketAddr;
use futures_util::{StreamExt, SinkExt};

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
    let socket_addr: SocketAddr = addr.parse().expect("Invalid address");

    // Serve the HTML file
    let html = warp::path::end()
        .and(warp::fs::file("html/index.html"));

    // WebSocket route
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .map(|ws: warp::ws::Ws| {
            ws.on_upgrade(handle_websocket)
        });

    // Combine routes
    let routes = html.or(ws_route);

    warp::serve(routes).run(socket_addr).await;
}

async fn handle_websocket(ws: warp::ws::WebSocket) {
    let (mut write, mut read) = ws.split();

    while let Some(result) = read.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    write.send(msg).await.expect("Failed to send message");
                }
            }
            Err(e) => {
                error!("Error processing message: {:?}", e);
                break;
            }
        }
    }
}