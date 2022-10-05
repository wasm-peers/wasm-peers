use axum::{extract::ws::WebSocketUpgrade, response::Response, routing::get, Extension, Router};

use crate::one_to_one::{user_connected, Connections, Sessions};

async fn handler(
    ws: WebSocketUpgrade,
    Extension(connections): Extension<Connections>,
    Extension(sessions): Extension<Sessions>,
) -> Response {
    ws.on_upgrade(move |socket| user_connected(socket, connections, sessions))
}

pub fn create_router() -> Router {
    let connections = Connections::default();
    let sessions = Sessions::default();
    Router::new()
        .route("/one_to_one", get(handler))
        .layer(Extension(connections))
        .layer(Extension(sessions))
}
