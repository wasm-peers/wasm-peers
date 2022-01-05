use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

pub type Connections = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

pub async fn user_connected(ws: WebSocket, connections: Connections) {
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new user: {}", id);

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| eprintln!("websocket send error: {}", e))
                .await;
        }
    });

    connections.write().await.insert(id, tx);

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error (id={}): {}", id, e);
                break;
            }
        };

        user_message(id, msg, &connections).await;
    }

    eprintln!("user disconnected: {}", id);
    connections.write().await.remove(&id);
}

#[derive(Deserialize)]
struct Request {}

#[derive(Serialize, Clone)]
struct Response {}

impl Response {
    fn new() -> Self {
        Response {}
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl ErrorResponse {
    fn new(error: String) -> Self {
        ErrorResponse { error }
    }
}

async fn user_message(id: usize, msg: Message, connections: &Connections) {
    if let Ok(msg) = msg.to_str() {
        match serde_json::from_str::<Request>(msg) {
            Ok(request) => {
                // TODO: create some meaningful response
                let response = Response::new();

                let response =
                    serde_json::to_string(&response).expect("failed to serialize response");

                for (_, tx) in connections.read().await.iter() {
                    // Ok - do nothing
                    // Err - tunnel was closed and is being removed from the connections dictionary
                    let _ = tx.send(Message::text(response.clone()));
                }
            }
            Err(e) => {
                if let Some(tx) = connections.read().await.get(&id) {
                    let response = serde_json::to_string(&ErrorResponse::new(e.to_string()))
                        .expect("failed to serialize error response");
                    let _ = tx.send(Message::text(response));
                }
            }
        }
    }
}
