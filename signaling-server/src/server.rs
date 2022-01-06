use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use log::{error, info};

use rusty_games_protocol::{SessionId, SignalMessage};

type UserId = usize;

pub struct Session {
    pub first: UserId,
    pub second: Option<UserId>,
}


pub type Connections = Arc<RwLock<HashMap<UserId, mpsc::UnboundedSender<Message>>>>;
pub type Sessions = Arc<RwLock<HashMap<SessionId, Session>>>;

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

pub async fn user_connected(ws: WebSocket, connections: Connections, sessions: Sessions) {
    let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
    info!("new user connected: {}", id);

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

        user_message(id, msg, &connections, &sessions).await;
    }

    eprintln!("user disconnected: {}", id);
    connections.write().await.remove(&id);
}

async fn user_message(user_id: UserId, msg: Message, connections: &Connections, sessions: &Sessions) {
    if let Ok(msg) = msg.to_str() {
        match serde_json::from_str::<SignalMessage>(msg) {
            Ok(request) => {
                match request {
                    // on first user in session - create session object and store connecting user id
                    // on second user - add him to existing session
                    SignalMessage::NewConnection(session_id) => {
                        match sessions.write().await.entry(session_id) {
                            Entry::Occupied(mut entry) => {
                                entry.get_mut().second = Some(user_id);
                            }
                            Entry::Vacant(entry) => {
                                entry.insert(Session { first: user_id, second: None });
                            }
                        }
                    }
                    // pass offer to the other user in session without changing anything
                    SignalMessage::SdpOffer(offer, session_id) => {
                        match sessions.read().await.get(&session_id) {
                            Some(session) => {
                                let recipient = if user_id == session.first { session.second } else { Some(session.first) };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::SdpOffer(offer, session_id);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx = connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response));
                                    }
                                    None => {
                                        error!("Missing second user in session: {}", &session_id);
                                    }
                                }
                            }
                            None => {
                                error!("No such session: {}", &session_id);
                            }
                        }
                    }
                    // pass answer to the other user in session without changing anything
                    SignalMessage::SdpAnswer(answer, session_id) => {
                        match sessions.read().await.get(&session_id) {
                            Some(session) => {
                                let recipient = if user_id == session.first { session.second } else { Some(session.first) };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::SdpAnswer(answer, session_id);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx = connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response));
                                    }
                                    None => {
                                        error!("Missing second user in session: {}", &session_id);
                                    }
                                }
                            }
                            None => {
                                error!("No such session: {}", &session_id);
                            }
                        }
                    }
                    SignalMessage::IceCandidate(candidate, session_id) => {
                        match sessions.read().await.get(&session_id) {
                            Some(session) => {
                                let recipient = if user_id == session.first { session.second } else { Some(session.first) };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::IceCandidate(candidate, session_id);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx = connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response));
                                    }
                                    None => {
                                        error!("Missing second user in session: {}", &session_id);
                                    }
                                }
                            }
                            None => {
                                error!("No such session: {}", &session_id);
                            }
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                if let Some(tx) = connections.read().await.get(&user_id) {
                    let response = serde_json::to_string(&SignalMessage::Error(e.to_string()))
                        .expect("failed to serialize error response");
                    tx.send(Message::text(response)).unwrap();
                }
            }
        }
    }
}
