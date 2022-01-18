use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{error, info, warn};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use rusty_games_protocol::{SessionId, SignalMessage, UserId};

#[derive(Default, Debug)]
pub struct Session {
    pub host: Option<UserId>,
    pub users: Vec<UserId>,
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
    user_disconnected(id, &connections, &sessions).await;
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
                    SignalMessage::SessionJoin(session_id, is_host) => {
                        let session = sessions.write().await.entry(session_id.clone()).or_insert(Session::default());
                        let connections_reader = connections.read().await;

                        if is_host && session.host.is_none() {
                            session.host = Some(user_id);
                            for client_id in &session.users {
                                // start connections with all already present users
                                let host_tx = connections_reader.get(&user_id).export("host not in connections");
                                let host_response =
                                    SignalMessage::SessionReady(session_id.clone(), client_id.clone());
                                let host_response =
                                    serde_json::to_string(&host_response).unwrap();
                                host_tx.send(Message::text(&host_response)).expect("failed to send SessionReady message to host");
                            }
                        } else if is_host && session.host.is_some() {
                            error!("connecting user wants to be a host, but host is already present!");
                            return;
                        } else {
                            // connect new user with host
                            session.users.push(user_id);

                            if let Some(host_id) = session.host {
                                let host_tx = connections_reader.get(&host_id).export("host not in connections");
                                let host_response =
                                    SignalMessage::SessionReady(session_id.clone(), user_id.clone());
                                let host_response =
                                    serde_json::to_string(&host_response).unwrap();
                                host_tx.send(Message::text(&host_response)).expect("failed to send SessionReady message to host");
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

                                let recipient = if Some(user_id) == session.first {
                                    session.second
                                } else {
                                    session.first
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
                                let recipient = if Some(user_id) == session.first {
                                    session.second
                                } else {
                                    session.first
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
                                let recipient = if Some(user_id) == session.first {
                                    session.second
                                } else {
                                    session.first
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

async fn user_disconnected(user_id: usize, connections: &Connections, sessions: &Sessions) {
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
