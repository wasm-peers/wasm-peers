use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use log::{error, info, warn};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};

use rusty_games_protocol::one_to_many::SignalMessage;
use rusty_games_protocol::{SessionId, UserId};

#[derive(Default, Debug)]
pub struct Session {
    pub host: Option<UserId>,
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
    sender_id: UserId,
    msg: Message,
    connections: &Connections,
    sessions: &Sessions,
) {
    if let Ok(msg) = msg.to_str() {
        match serde_json::from_str::<SignalMessage>(msg) {
            Ok(request) => {
                info!("message received from user {:?}: {:?}", sender_id, request);
                match request {
                    SignalMessage::SessionJoin(session_id, is_host) => {
                        let mut sessions_writer = sessions.write().await;
                        let session = sessions_writer
                            .entry(session_id.clone())
                            .or_insert_with(Session::default);
                        let connections_reader = connections.read().await;

                        if is_host && session.host.is_none() {
                            session.host = Some(sender_id);
                            // start connections with all already present users
                            for client_id in &session.users {
                                {
                                    let host_tx = connections_reader
                                        .get(&sender_id)
                                        .expect("host not in connections");
                                    let host_response =
                                        SignalMessage::SessionReady(session_id.clone(), *client_id);
                                    let host_response =
                                        serde_json::to_string(&host_response).unwrap();
                                    host_tx
                                        .send(Message::text(&host_response))
                                        .expect("failed to send SessionReady message to host");
                                }
                                // {
                                //     let client_tx = connections_reader
                                //         .get(&client_id)
                                //         .expect("host not in connections");
                                //     let client_response =
                                //         SignalMessage::SessionReady(session_id.clone(), sender_id);
                                //     let client_response = serde_json::to_string(&client_response).unwrap();
                                //     client_tx
                                //         .send(Message::text(&client_response))
                                //         .expect("failed to send SessionReady message to host");
                                // }
                            }
                        } else if is_host && session.host.is_some() {
                            error!(
                                "connecting user wants to be a host, but host is already present!"
                            );
                        } else {
                            // connect new user with host
                            session.users.insert(sender_id);

                            if let Some(host_id) = session.host {
                                let host_tx = connections_reader
                                    .get(&host_id)
                                    .expect("host not in connections");
                                let host_response =
                                    SignalMessage::SessionReady(session_id.clone(), sender_id);
                                let host_response = serde_json::to_string(&host_response).unwrap();
                                host_tx
                                    .send(Message::text(&host_response))
                                    .expect("failed to send SessionReady message to host");
                            }
                        }
                    }
                    // pass offer to the other user in session without changing anything
                    SignalMessage::SdpOffer(session_id, recipient_id, offer) => {
                        let response = SignalMessage::SdpOffer(session_id, sender_id, offer);
                        let response = serde_json::to_string(&response).unwrap();
                        let connections_reader = connections.read().await;
                        if let Some(recipient_tx) = connections_reader.get(&recipient_id) {
                            recipient_tx.send(Message::text(response)).unwrap();
                        } else {
                            warn!("tried to send offer to non existing user");
                        }
                    }
                    // pass answer to the other user in session without changing anything
                    SignalMessage::SdpAnswer(session_id, recipient_id, answer) => {
                        let session_reader = sessions.read().await;
                        let host_id = session_reader
                            .get(&session_id)
                            .expect("no session id for requested SdpAnswer")
                            .host
                            .expect("no host for requested SdpAnswer");
                        let response = SignalMessage::SdpAnswer(session_id, sender_id, answer);
                        let response = serde_json::to_string(&response).unwrap();
                        let connections_reader = connections.read().await;
                        if let Some(recipient_tx) = connections_reader.get(&recipient_id) {
                            recipient_tx.send(Message::text(response)).unwrap();
                        } else {
                            warn!("tried to send offer to non existing user");
                        }
                    }
                    SignalMessage::IceCandidate(session_id, recipient_id, candidate) => {
                        let response =
                            SignalMessage::IceCandidate(session_id, sender_id, candidate);
                        let response = serde_json::to_string(&response).unwrap();
                        let connections_reader = connections.read().await;
                        let recipient_tx = connections_reader.get(&recipient_id).unwrap();

                        recipient_tx.send(Message::text(response)).unwrap();
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
    connections.write().await.remove(&user_id);

    let mut session_to_delete = None;
    for (session_id, session) in sessions.write().await.iter_mut() {
        if session.host == Some(user_id) {
            session.host = None;
        } else if session.users.contains(&user_id) {
            session.users.remove(&user_id);
        }
        if session.host == None && session.users.is_empty() {
            session_to_delete = Some(session_id.clone());
            break;
        }
    }
    // remove session if it's empty
    if let Some(session_id) = session_to_delete {
        sessions.write().await.remove(&session_id);
    }
}
