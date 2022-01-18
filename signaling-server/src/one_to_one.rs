use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{error, info, warn};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use rusty_games_protocol::{SessionId, UserId};
use rusty_games_protocol::one_to_one::SignalMessage;

pub struct Session {
    pub first: Option<UserId>,
    pub second: Option<UserId>,
    pub offer_received: bool,
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
                .unwrap_or_else(|e| eprintln!("websocket send error: {}", e))
                .await;
        }
    });

    connections.write().await.insert(user_id, tx);

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(e) => {
                eprintln!("websocket error (id={:?}): {}", user_id, e);
                break;
            }
        };

        user_message(user_id, msg, &connections, &sessions).await;
    }

    eprintln!("user disconnected: {:?}", user_id);
    user_disconnected(user_id, &connections, &sessions).await;
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
                info!("message received from user {:?}: {:?}", user_id, request);
                match request {
                    SignalMessage::SessionJoin(session_id) => {
                        match sessions.write().await.entry(session_id.clone()) {
                            // on first user in session - create session object and store connecting user id
                            Entry::Vacant(entry) => {
                                entry.insert(Session {
                                    first: Some(user_id),
                                    second: None,
                                    offer_received: false,
                                });
                            }
                            // on second user - add him to existing session and notify users that session is ready
                            Entry::Occupied(mut entry) => {
                                entry.get_mut().second = Some(user_id);
                                let first_response =
                                    SignalMessage::SessionReady(session_id.clone(), true);
                                let first_response =
                                    serde_json::to_string(&first_response).unwrap();
                                let second_response =
                                    SignalMessage::SessionReady(session_id, false);
                                let second_response =
                                    serde_json::to_string(&second_response).unwrap();

                                let connections_reader = connections.read().await;
                                if let Some(first_id) = &entry.get().first {
                                    let first_tx = connections_reader.get(first_id).unwrap();
                                    first_tx.send(Message::text(first_response)).unwrap();
                                    let second_tx = connections_reader.get(&user_id).unwrap();
                                    second_tx.send(Message::text(&second_response)).unwrap();
                                }
                            }
                        }
                    }
                    // pass offer to the other user in session without changing anything
                    SignalMessage::SdpOffer(session_id, offer) => {
                        match sessions.write().await.get_mut(&session_id) {
                            Some(session) => {
                                if session.offer_received {
                                    warn!("offer already sent by the the peer, ignoring the second offer: {:?}", session_id);
                                } else {
                                    session.offer_received = true;
                                }

                                let recipient = if Some(user_id) == session.first {
                                    session.second
                                } else {
                                    session.first
                                };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::SdpOffer(session_id, offer);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx =
                                            connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response)).unwrap();
                                    }
                                    None => {
                                        error!("Missing second user in session: {:?}", &session_id);
                                    }
                                }
                            }
                            None => {
                                error!("No such session: {:?}", &session_id);
                            }
                        }
                    }
                    // pass answer to the other user in session without changing anything
                    SignalMessage::SdpAnswer(session_id, answer) => {
                        match sessions.read().await.get(&session_id) {
                            Some(session) => {
                                let recipient = if Some(user_id) == session.first {
                                    session.second
                                } else {
                                    session.first
                                };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response = SignalMessage::SdpAnswer(session_id, answer);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx =
                                            connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response)).unwrap();
                                    }
                                    None => {
                                        error!("Missing second user in session: {:?}", &session_id);
                                    }
                                }
                            }
                            None => {
                                error!("No such session: {:?}", &session_id);
                            }
                        }
                    }
                    SignalMessage::IceCandidate(session_id, candidate) => {
                        match sessions.read().await.get(&session_id) {
                            Some(session) => {
                                let recipient = if Some(user_id) == session.first {
                                    session.second
                                } else {
                                    session.first
                                };
                                match recipient {
                                    Some(recipient_id) => {
                                        let response =
                                            SignalMessage::IceCandidate(session_id, candidate);
                                        let response = serde_json::to_string(&response).unwrap();
                                        let connections_reader = connections.read().await;
                                        let recipient_tx =
                                            connections_reader.get(&recipient_id).unwrap();

                                        recipient_tx.send(Message::text(response)).unwrap();
                                    }
                                    None => {
                                        error!("Missing second user in session: {:?}", &session_id);
                                    }
                                }
                            }
                            None => {
                                error!("No such session: {:?}", &session_id);
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

async fn user_disconnected(user_id: UserId, connections: &Connections, sessions: &Sessions) {
    let mut session_to_delete = None;
    for (session_id, session) in sessions.write().await.iter_mut() {
        if session.first == Some(user_id) {
            session.first = None;
        } else if session.second == Some(user_id) {
            session.second = None;
        }
        if session.first == None && session.second == None {
            session_to_delete = Some(session_id.clone());
        }
    }
    // remove session if it's empty
    if let Some(session_id) = session_to_delete {
        sessions.write().await.remove(&session_id);
    }
    connections.write().await.remove(&user_id);
}
