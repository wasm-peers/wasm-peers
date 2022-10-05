use axum::{extract::ws::WebSocketUpgrade, response::Response, routing::get, Extension, Router};
use std::net::SocketAddr;
use wasm_peers_signaling_server_axum::one_to_one::{self, Connections, Sessions};

#[tokio::main]
async fn main() {
    let connections = one_to_one::Connections::default();
    let sessions = one_to_one::Sessions::default();

    let app = Router::new()
        .route("/one_to_one", get(handler))
        .layer(Extension(connections))
        .layer(Extension(sessions));

    async fn handler(
        ws: WebSocketUpgrade,
        Extension(connections): Extension<Connections>,
        Extension(sessions): Extension<Sessions>,
    ) -> Response {
        ws.on_upgrade(move |socket| one_to_one::user_connected(socket, connections, sessions))
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 9001));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
