use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::protocol::Message as TungsteniteMessage;
use futures_util::{StreamExt, SinkExt};
use log::{info, error};
use serde::Deserialize;
use warp::Filter;

#[derive(Deserialize)]
struct Config {
    address: String,
    port: u16,
}

type Tx = mpsc::UnboundedSender<TungsteniteMessage>;
type PeerMap = Arc<Mutex<HashMap<usize, Tx>>>;

#[tokio::main]
async fn main() {
    // Initialiser logning
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    info!("Starter server...");
    println!("Starter server...");

    // Læs konfiguration fra config.toml
    let config: Config = toml::from_str(&fs::read_to_string("config.toml").expect("Kunne ikke læse konfigurationsfil"))
        .expect("Kunne ikke parse konfigurationsfil");

    let addr = format!("{}:{}", config.address, config.port);
    let socket_addr: SocketAddr = addr.parse().expect("Ugyldig adresse");

    // Delte datastrukturer til klientforbindelser
    let peers = PeerMap::new(Mutex::new(HashMap::new()));
    let peer_id_counter = Arc::new(Mutex::new(0));

    // Server HTML
    let html = warp::path::end()
        .and(warp::fs::file("html/index.html"))
        .map(|file| {
            info!("Serverer index.html");
            file
        });

    // WebSocket-rute
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(with_peers(peers.clone()))
        .and(with_peer_id_counter(peer_id_counter.clone()))
        .map(|ws: warp::ws::Ws, peers, peer_id_counter| {
            ws.on_upgrade(move |socket| handle_websocket(socket, peers, peer_id_counter))
        });

    // Kombiner ruter og start serveren
    let routes = html.or(ws_route);
    warp::serve(routes).run(socket_addr).await;
}

// Hjælpefunktion til deling af peers
fn with_peers(peers: PeerMap) -> impl Filter<Extract = (PeerMap,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peers.clone())
}

// Hjælpefunktion til deling af peer-id-tæller
fn with_peer_id_counter(peer_id_counter: Arc<Mutex<usize>>) -> impl Filter<Extract = (Arc<Mutex<usize>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peer_id_counter.clone())
}

// WebSocket-håndtering
async fn handle_websocket(ws: warp::ws::WebSocket, peers: PeerMap, peer_id_counter: Arc<Mutex<usize>>) {
    let (mut write, mut read) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Tildel unikt ID til klienten
    let peer_id = {
        let mut id_counter = peer_id_counter.lock().unwrap();
        *id_counter += 1;
        *id_counter
    };

    {
        let mut peers = peers.lock().unwrap();
        peers.insert(peer_id, tx.clone());
    }

    // Task til at sende beskeder til klienten
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let warp_msg = match msg {
                TungsteniteMessage::Text(text) => warp::ws::Message::text(text),
                TungsteniteMessage::Binary(bin) => warp::ws::Message::binary(bin),
                _ => continue,
            };
            if write.send(warp_msg).await.is_err() {
                break;
            }
        }
    });

    // Læs beskeder fra klienten og broadcast til alle
    while let Some(result) = read.next().await {
        match result {
            Ok(msg) => {
                let tungstenite_msg = if msg.is_text() {
                    // Behold som tekst
                    TungsteniteMessage::Text(msg.to_str().unwrap_or_default().to_string())
                } else if msg.is_binary() {
                    // Behold som binær
                    TungsteniteMessage::Binary(msg.into_bytes())
                } else {
                    // Ignorer andre typer
                    continue;
                };

                // Send til alle andre klienter
                let peers = peers.lock().unwrap();
                for peer in peers.values() {
                    let _ = peer.send(tungstenite_msg.clone());
                }
            }
            Err(e) => {
                error!("Fejl ved behandling af besked: {:?}", e);
                break;
            }
        }
    }

    // Fjern klienten når forbindelsen lukkes
    let mut peers = peers.lock().unwrap();
    peers.remove(&peer_id);
}