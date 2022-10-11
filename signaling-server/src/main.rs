use std::net::SocketAddr;

use wasm_peers_signaling_server_axum::router::create_router;

#[tokio::main]
async fn main() {
    let app = create_router();

    let addr = SocketAddr::from(([127, 0, 0, 1], 9001));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
