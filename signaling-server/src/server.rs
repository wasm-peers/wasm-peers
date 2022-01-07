use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{error, info, warn};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use rusty_games_protocol::{SessionId, SignalMessage};

type UserId = usize;

pub struct Session {
    pub first: UserId,
    pub second: Option<UserId>,
    pub offer_received: bool,
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

async fn user_message(
    user_id: UserId,
    msg: Message,
    connections: &Connections,
    sessions: &Sessions,
) {
    if let Ok(msg) = msg.to_str() {
        match serde_json::from_str::<SignalMessage>(msg) {
            Ok(request) => {
                info!("message received from user {}: {:?}", user_id, request);
                match request {
                    SignalMessage::SessionStartOrJoin(session_id) => {
                        match sessions.write().await.entry(session_id.clone()) {
                            // on first user in session - create session object and store connecting user id
                            Entry::Vacant(entry) => {
                                entry.insert(Session {
                                    first: user_id,
                                    second: None,
                                    offer_received: false,
                                });
                            }
                            // on second user - add him to existing session and notify users that session is ready
                            Entry::Occupied(mut entry) => {
                                entry.get_mut().second = Some(user_id);
                                let response = SignalMessage::SessionReady(session_id);
                                let response = serde_json::to_string(&response).unwrap();
                                let connections_reader = connections.read().await;
                                let recipient_1_tx = connections_reader.get(&user_id).unwrap();
                                let recipient_2_tx =
                                    connections_reader.get(&entry.get().first).unwrap();

                                recipient_1_tx.send(Message::text(&response)).unwrap();
                                recipient_2_tx.send(Message::text(response)).unwrap();
                            }
                        }
                    }
                    // pass offer to the other user in session without changing anything
                    SignalMessage::SdpOffer(offer, session_id) => {
                        match sessions.write().await.get_mut(&session_id) {
                            Some(session) => {
                                if session.offer_received {
                                    warn!("offer already sent by the the peer, ignoring the second offer: {}", session_id);
                                } else {
                                    session.offer_received = true;
                                }

                                let recipient = if user_id == session.first {
                                    session.second
                                } else {
                                    Some(session.first)
                                };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::SdpOffer(offer, session_id);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx =
                                            connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response)).unwrap();
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
                                let recipient = if user_id == session.first {
                                    session.second
                                } else {
                                    Some(session.first)
                                };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::SdpAnswer(answer, session_id);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx =
                                            connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response)).unwrap();
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
                                let recipient = if user_id == session.first {
                                    session.second
                                } else {
                                    Some(session.first)
                                };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response =
                                            SignalMessage::IceCandidate(candidate, session_id);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx =
                                            connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response)).unwrap();
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
            Err(error) => {
                error!("An error occurred: {:?}", error);
            }
        }
    }
}
