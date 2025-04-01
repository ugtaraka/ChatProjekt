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
    // Initialiser logning fra log4rs.yaml filen
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    info!("Starter server...");
    println!("Starter server...");

    // Læs konfiguration fra config.toml
    let config: Config = toml::from_str(&fs::read_to_string("config.toml").expect("Kunne ikke læse konfigurationsfil"))
        .expect("Kunne ikke parse konfigurationsfil");

    let addr = format!("{}:{}", config.address, config.port);
    let socket_addr: SocketAddr = addr.parse().expect("Ugyldig adresse");

    // Delt tilstand for tilsluttede klienter
    let peers = PeerMap::new(Mutex::new(HashMap::new()));
    let peer_id_counter = Arc::new(Mutex::new(0));

    // Server HTML-filen
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

    // Kombiner ruter
    let routes = html.or(ws_route);

    // Start serveren
    warp::serve(routes).run(socket_addr).await;
}

fn with_peers(peers: PeerMap) -> impl Filter<Extract = (PeerMap,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peers.clone())
}

fn with_peer_id_counter(peer_id_counter: Arc<Mutex<usize>>) -> impl Filter<Extract = (Arc<Mutex<usize>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || peer_id_counter.clone())
}

// Håndter WebSocket-forbindelser
async fn handle_websocket(ws: warp::ws::WebSocket, peers: PeerMap, peer_id_counter: Arc<Mutex<usize>>) {
    let (mut write, mut read) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    let peer_id = {
        let mut id_counter = peer_id_counter.lock().unwrap();
        *id_counter += 1;
        *id_counter
    };

    {
        let mut peers = peers.lock().unwrap();
        peers.insert(peer_id, tx.clone());
    }

    // Opgave til at videresende beskeder fra kanalen til WebSocket
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(warp::ws::Message::binary(msg.into_data())).await.is_err() {
                break;
            }
        }
    });

    // Læs og broadcast beskeder asynkront
    while let Some(result) = read.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    let tungstenite_msg = TungsteniteMessage::binary(msg.into_bytes());
                    let peers = peers.lock().unwrap();
                    for peer in peers.values() {
                        let _ = peer.send(tungstenite_msg.clone());
                    }
                }
            }
            Err(e) => {
                error!("Fejl ved behandling af besked: {:?}", e);
                break;
            }
        }
    }

    // Fjern klienten fra peer-sættet
    let mut peers = peers.lock().unwrap();
    peers.remove(&peer_id);
}