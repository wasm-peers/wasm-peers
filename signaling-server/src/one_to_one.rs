use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::anyhow;
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use wasm_peers_protocol::one_to_one::SignalMessage;
use wasm_peers_protocol::{SessionId, UserId};

pub struct Session {
    pub first: Option<UserId>,
    pub second: Option<UserId>,
    pub offer_received: bool,
}

pub type Connections = Arc<RwLock<HashMap<UserId, mpsc::UnboundedSender<Message>>>>;
pub type Sessions = Arc<RwLock<HashMap<SessionId, Session>>>;

static NEXT_USER_ID: AtomicU64 = AtomicU64::new(1);

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
                .await
                .unwrap_or_else(|e| error!("websocket send error: {}", e));
        }
    });

    connections.write().await.insert(user_id, tx);

    while let Some(result) = user_ws_rx.next().await {
        let msg = match result {
            Ok(msg) => msg,
            Err(err) => {
                error!("websocket error (user_id={:?}): {}", user_id, err);
                break;
            }
        };

        if let Err(err) = user_message(user_id, msg, &connections, &sessions).await {
            error!("user_message error: {}", err);
        }
    }

    error!("user disconnected: {:?}", user_id);
    user_disconnected(user_id, &connections, &sessions).await;
}

async fn user_message(
    user_id: UserId,
    msg: Message,
    connections: &Connections,
    sessions: &Sessions,
) -> crate::Result<()> {
    let request = rmp_serde::from_slice::<SignalMessage>(msg.into_data().as_ref())?;
    info!("message received from user {:?}: {:?}", user_id, request);
    match request {
        SignalMessage::SessionJoin(session_id) => {
            session_join(sessions, connections, user_id, session_id).await?;
        }
        // pass offer to the other user in session without changing anything
        SignalMessage::SdpOffer(session_id, offer) => {
            sdp_offer(sessions, connections, user_id, session_id, offer).await?;
        }
        // pass answer to the other user in session without changing anything
        SignalMessage::SdpAnswer(session_id, answer) => {
            let sessions = sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| anyhow!("no such session: {:?}", &session_id))?;
            let recipient_id = if Some(user_id) == session.first {
                session.second
            } else {
                session.first
            }
            .ok_or_else(|| anyhow!("missing second user in session: {:?}", &session_id))?;
            let response = SignalMessage::SdpAnswer(session_id, answer);
            let response = rmp_serde::to_vec(&response)?;
            let connections_reader = connections.read().await;
            connections_reader
                .get(&recipient_id)
                .ok_or_else(|| anyhow!("no sender for given recipient_id"))?
                .send(Message::Binary(response))?;
        }
        SignalMessage::IceCandidate(session_id, candidate) => {
            let sessions = sessions.read().await;
            let session = sessions
                .get(&session_id)
                .ok_or_else(|| anyhow!("no such session: {:?}", &session_id))?;
            let recipient_id = if Some(user_id) == session.first {
                session.second
            } else {
                session.first
            }
            .ok_or_else(|| anyhow!("missing second user in session: {:?}", &session_id))?;
            let response = SignalMessage::IceCandidate(session_id, candidate);
            let response = rmp_serde::to_vec(&response)?;
            let connections_reader = connections.read().await;
            connections_reader
                .get(&recipient_id)
                .ok_or_else(|| anyhow!("no sender for given recipient_id"))?
                .send(Message::Binary(response))?;
        }
        other => {
            error!("received unexpected signal message: {:?}", other);
        }
    }
    Ok(())
}

async fn session_join(
    sessions: &Sessions,
    connections: &Connections,
    user_id: UserId,
    session_id: SessionId,
) -> crate::Result<()> {
    let mut sessions = sessions.write().await;
    match sessions.entry(session_id) {
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
            let first_response = SignalMessage::SessionReady(session_id, true);
            let first_response = rmp_serde::to_vec(&first_response)?;
            let second_response = SignalMessage::SessionReady(session_id, false);
            let second_response = rmp_serde::to_vec(&second_response)?;

            let connections_reader = connections.read().await;
            if let Some(first_id) = entry.get().first {
                connections_reader
                    .get(&first_id)
                    .ok_or_else(|| anyhow!("no sender for given id"))?
                    .send(Message::Binary(first_response))?;
                connections_reader
                    .get(&user_id)
                    .ok_or_else(|| anyhow!("no sender for given id"))?
                    .send(Message::Binary(second_response))?;
            }
        }
    }
    Ok(())
}

async fn sdp_offer(
    sessions: &Sessions,
    connections: &Connections,
    user_id: UserId,
    session_id: SessionId,
    offer: String,
) -> crate::Result<()> {
    let mut sessions = sessions.write().await;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| anyhow!("no such session: {:?}", &session_id))?;
    if session.offer_received {
        info!(
            "offer already sent by the the peer, ignoring the second offer: {:?}",
            session_id
        );
    } else {
        session.offer_received = true;
    }

    let recipient_id = if Some(user_id) == session.first {
        session.second
    } else {
        session.first
    }
    .ok_or_else(|| anyhow!("missing second user in session: {:?}", &session_id))?;
    let response = SignalMessage::SdpOffer(session_id, offer);
    let response = rmp_serde::to_vec(&response)?;
    let connections_reader = connections.read().await;
    connections_reader
        .get(&recipient_id)
        .ok_or_else(|| anyhow!("no sender for given recipient_id"))?
        .send(Message::Binary(response))?;
    Ok(())
}

async fn user_disconnected(user_id: UserId, connections: &Connections, sessions: &Sessions) {
    connections.write().await.remove(&user_id);

    let mut sessions = sessions.write().await;
    let mut session_to_delete = None;
    for (session_id, session) in sessions.iter_mut() {
        if session.first == Some(user_id) {
            session.first = None;
            if session.first.is_none() && session.second.is_none() {
                session_to_delete = Some(*session_id);
            }
            break;
        } else if session.second == Some(user_id) {
            session.second = None;
            if session.first.is_none() && session.second.is_none() {
                session_to_delete = Some(*session_id);
            }
            break;
        }
    }
    // remove session if it's empty
    if let Some(session_id) = session_to_delete {
        sessions.remove(&session_id);
    }
}
