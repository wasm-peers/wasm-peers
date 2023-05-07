use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{error, info, warn};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use wasm_peers_protocol::one_to_many::SignalMessage;
use wasm_peers_protocol::{SessionId, UserId};

#[derive(Default, Debug)]
pub struct Session {
    pub users: HashSet<UserId>,
}

pub type Connections = Arc<RwLock<HashMap<UserId, mpsc::UnboundedSender<Message>>>>;
pub type Sessions = Arc<RwLock<HashMap<SessionId, Session>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

pub async fn user_connected(ws: WebSocket, connections: Connections, sessions: Sessions) {
    let user_id = UserId::new(NEXT_USER_ID.fetch_add(1, Ordering::Relaxed));
    info!("new user connected: {:?}", user_id);

    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let (tx, rx) = mpsc::unbounded_channel();
    let mut rx = UnboundedReceiverStream::new(rx);

    tokio::task::spawn(async move {
        while let Some(message) = rx.next().await {
            user_ws_tx
                .send(message)
                .unwrap_or_else(|e| error!("websocket send error: {}", e))
                .await;
        }
    });

    connections.write().await.insert(user_id, tx);

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                error!("websocket error (id={:?}): {}", user_id, e);
                break;
            }
        };

        if let Err(err) = user_message(user_id, msg, &connections, &sessions).await {
            error!("error while handling user message: {}", err);
        }
    }

    error!("user disconnected: {:?}", user_id);
    user_disconnected(user_id, &connections, &sessions).await;
}

async fn user_message(
    sender_id: UserId,
    msg: Message,
    connections: &Connections,
    sessions: &Sessions,
) -> crate::Result<()> {
    if let Ok(msg) = msg.to_text() {
        match serde_json::from_str::<SignalMessage>(msg) {
            Ok(request) => {
                info!("message received from user {:?}: {:?}", sender_id, request);
                match request {
                    SignalMessage::SessionJoin(session_id, _) => {
                        let mut sessions_writer = sessions.write().await;
                        let session = sessions_writer
                            .entry(session_id.clone())
                            .or_insert_with(Session::default);
                        let connections_reader = connections.read().await;

                        // start connections with all already present users
                        for client_id in &session.users {
                            {
                                let host_tx = connections_reader
                                    .get(&sender_id)
                                    .expect("host not in connections");
                                let host_response =
                                    SignalMessage::SessionReady(session_id.clone(), *client_id);
                                let host_response = serde_json::to_string(&host_response)?;
                                host_tx
                                    .send(Message::Text(host_response))
                                    .expect("failed to send SessionReady message to host");
                            }
                        }
                        session.users.insert(sender_id);
                    }
                    // pass offer to the other user in session without changing anything
                    SignalMessage::SdpOffer(session_id, recipient_id, offer) => {
                        let response = SignalMessage::SdpOffer(session_id, sender_id, offer);
                        let response = serde_json::to_string(&response)?;
                        let connections_reader = connections.read().await;
                        if let Some(recipient_tx) = connections_reader.get(&recipient_id) {
                            recipient_tx.send(Message::Text(response))?;
                        } else {
                            warn!("tried to send offer to non existing user");
                        }
                    }
                    // pass answer to the other user in session without changing anything
                    SignalMessage::SdpAnswer(session_id, recipient_id, answer) => {
                        let response = SignalMessage::SdpAnswer(session_id, sender_id, answer);
                        let response = serde_json::to_string(&response)?;
                        let connections_reader = connections.read().await;
                        if let Some(recipient_tx) = connections_reader.get(&recipient_id) {
                            recipient_tx.send(Message::Text(response))?;
                        } else {
                            warn!("tried to send offer to non existing user");
                        }
                    }
                    SignalMessage::IceCandidate(session_id, recipient_id, candidate) => {
                        let response =
                            SignalMessage::IceCandidate(session_id, sender_id, candidate);
                        let response = serde_json::to_string(&response)?;
                        let connections_reader = connections.read().await;
                        let recipient_tx = connections_reader
                            .get(&recipient_id)
                            .ok_or_else(|| anyhow!("no sender for given id"))?;

                        recipient_tx.send(Message::Text(response))?;
                    }
                    _ => {}
                }
            }
            Err(error) => {
                error!("An error occurred: {:?}", error);
            }
        }
    }
    Ok(())
}

async fn user_disconnected(user_id: UserId, connections: &Connections, sessions: &Sessions) {
    connections.write().await.remove(&user_id);

    let mut session_to_delete = None;
    let mut sessions = sessions.write().await;
    for (session_id, session) in sessions.iter_mut() {
        if session.users.contains(&user_id) {
            session.users.remove(&user_id);
        }
        if session.users.is_empty() {
            session_to_delete = Some(session_id.clone());
            break;
        }
    }
    // remove session if it's empty
    if let Some(session_id) = session_to_delete {
        sessions.remove(&session_id);
    }
}
