use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use axum::Router;

use crate::{many_to_many, one_to_many, one_to_one};

#[derive(Default, Clone)]
pub struct ServerState {
    one_to_one_connections: one_to_one::Connections,
    one_to_one_sessions: one_to_one::Sessions,
    one_to_many_connections: one_to_many::Connections,
    one_to_many_sessions: one_to_many::Sessions,
    many_to_many_connections: many_to_many::Connections,
    many_to_many_sessions: many_to_many::Sessions,
}

#[allow(clippy::unused_async)]
async fn health_handler() -> &'static str {
    "OK"
}

#[allow(clippy::unused_async)]
async fn one_to_one_handler(State(state): State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| {
        one_to_one::user_connected(
            socket,
            state.one_to_one_connections,
            state.one_to_one_sessions,
        )
    })
}

#[allow(clippy::unused_async)]
async fn one_to_many_handler(State(state): State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| {
        one_to_many::user_connected(
            socket,
            state.one_to_many_connections,
            state.one_to_many_sessions,
        )
    })
}

#[allow(clippy::unused_async)]
async fn many_to_many_handler(State(state): State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(move |socket| {
        many_to_many::user_connected(
            socket,
            state.many_to_many_connections,
            state.many_to_many_sessions,
        )
    })
}

pub fn create(server_state: ServerState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/one-to-one", get(one_to_one_handler))
        .route("/one-to-many", get(one_to_many_handler))
        .route("/many-to-many", get(many_to_many_handler))
        .with_state(server_state)
}
