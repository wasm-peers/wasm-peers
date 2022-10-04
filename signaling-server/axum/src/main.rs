use axum::{
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::env;
use std::net::SocketAddr;
use std::str::FromStr;
//use wasm_peers_signaling_server::{many_to_many, one_to_many, one_to_one};

#[tokio::main]
async fn main() {
    let app = Router::new().route("/ws", get(handler));

    async fn handler(ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(handle_socket)
    }

    async fn handle_socket(mut socket: WebSocket) {
        while let Some(msg) = socket.recv().await {
            println!("recieved");
            let msg = if let Ok(msg) = msg {
                msg
            } else {
                // client disconnected
                println!("disconnected");
                return;
            };

            if socket.send(msg.clone()).await.is_err() {
                // client disconnected
                return;
            }

            println!("send {:#?}", msg);
        }
    }

    // run it with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 9001));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
