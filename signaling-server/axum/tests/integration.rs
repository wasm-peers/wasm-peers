use std::net::TcpListener;
use std::sync::Arc;

use wasm_peers::one_to_one::NetworkManager;
use wasm_peers::{ConnectionType, SessionId};
use wasm_peers_signaling_server_axum::router::create_router;

const STUN_URL: &str = "stun.l.google.com:19302";

#[tokio::test]
async fn test_clients_with_axum_server() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();

    let routing = create_router();
    let server = axum::Server::from_tcp(listener)
        .unwrap()
        .serve(routing.into_make_service());

    tokio::spawn(server);

    let address = Arc::new(format!("http://127.0.0.1:{}", port));

    let session_id = SessionId::new("some-session-id".to_string());
    let mut peer1 = NetworkManager::new(
        address.as_str(),
        session_id.clone(),
        ConnectionType::Stun {
            urls: STUN_URL.to_string(),
        },
    )
    .unwrap();

    let peer1_clone = peer1.clone();
    let peer1_on_open = move || peer1_clone.send_message("ping!").unwrap();
    let peer1_on_message = {
        move |message| {
            println!("peer1 received message: {}", message);
        }
    };
    peer1.start(peer1_on_open, peer1_on_message).unwrap();

    let mut peer2 = NetworkManager::new(
        address.as_str(),
        session_id,
        ConnectionType::Stun {
            urls: STUN_URL.to_string(),
        },
    )
    .unwrap();
    let peer2_on_open = || { /* do nothing */ };
    let peer2_clone = peer2.clone();
    let peer2_on_message = {
        move |message| {
            println!("peer2 received message: {}", message);
            peer2_clone.send_message("pong!").unwrap();
        }
    };
    peer2.start(peer2_on_open, peer2_on_message).unwrap();
}
