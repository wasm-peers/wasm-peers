/*!
Library module for one-to-one network topology in simple tunnel connection.

After connection is established both peers are treated equally and have an opportunity to send messages
with [`NetworkManager::send_message`] method.

# Example

This example shows two peers sending `ping` and `pong` messages to each other.

```
use wasm_peers::{ConnectionType, SessionId};
use wasm_peers::one_to_one::NetworkManager;
use web_sys::console;

const SIGNALING_SERVER_URL: &str = "ws://0.0.0.0:9001/one-to-one";
const STUN_SERVER_URL: &str = "stun:openrelay.metered.ca:80";

let session_id = SessionId::new("some-session-id".to_string());
let mut peer1 = NetworkManager::new(
    SIGNALING_SERVER_URL,
    session_id.clone(),
    &ConnectionType::Stun { urls: STUN_SERVER_URL.to_string() },
)
.unwrap();

let peer1_clone = peer1.clone();
let peer1_on_open = move || peer1_clone.send_message("ping!").unwrap();
let peer1_on_message = {
    move |message| {
        console::log_1(&format!("peer1 received message: {}", message).into());
    }
};
peer1.start(peer1_on_open, peer1_on_message).unwrap();

let mut peer2 = NetworkManager::new(
    SIGNALING_SERVER_URL,
    session_id,
    &ConnectionType::Stun { urls: STUN_SERVER_URL.to_string() },
)
.unwrap();
let peer2_on_open = || { /* do nothing */ };
let peer2_clone = peer2.clone();
let peer2_on_message = {
    move |message| {
        console::log_1(&format!("peer2 received message: {}", message).into());
        peer2_clone.send_message("pong!").unwrap();
    }
};
peer2.start(peer2_on_open, peer2_on_message).unwrap();
```
*/

use std::cell::RefCell;
use std::rc::Rc;

use anyhow::anyhow;
use log::debug;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_peers_protocol::SessionId;
use web_sys::{RtcDataChannel, RtcDataChannelInit, RtcPeerConnection, WebSocket};

use crate::constants::DEFAULT_MAX_RETRANSMITS;
use crate::one_to_one::callbacks::{
    set_data_channel_on_error, set_data_channel_on_message, set_data_channel_on_open,
    set_peer_connection_on_data_channel, set_peer_connection_on_ice_candidate,
    set_peer_connection_on_ice_connection_state_change, set_websocket_on_message,
    set_websocket_on_open,
};
use crate::utils::{
    create_peer_connection, set_peer_connection_on_ice_gathering_state_change,
    set_peer_connection_on_negotiation_needed, ConnectionType,
};

mod callbacks;
mod websocket_handler;

#[derive(Debug, Clone)]
pub struct NetworkManagerInner {
    session_id: SessionId,
    websocket: WebSocket,
    peer_connection: RtcPeerConnection,
    pub data_channel: Option<RtcDataChannel>,
}

/// Abstraction over `WebRTC` peer-to-peer connection.
/// Structure representing one of two equal peers.
///
/// `WebRTC` data channel communication abstracted to a single class.
/// All setup is handled internally, you must only provide callbacks
/// for when the connection opens and for handling incoming messages.
/// It also provides a method of sending data to the other end of the connection.
///
/// Only works with [wasm-peers-signaling-server](https://docs.rs/wasm-peers-signaling-server/latest/wasm_peers_signaling_server/) instance,
/// whose full  address must be provided.
///
/// Start-up flow is divided into two methods [`NetworkManager::new`] and [`NetworkManager::start`]
/// to allow possibility of referring to network manger itself from the callbacks.
///
/// This class is a  pointer to the underlying resource and can be cloned freely.
#[derive(Debug, Clone)]
pub struct NetworkManager {
    pub inner: Rc<RefCell<NetworkManagerInner>>,
}

impl NetworkManager {
    /// Creates an instance with all resources required to create a connection.
    /// Requires an  address of an signaling server instance,
    /// session id by which it will identify connecting pair of peers and type of connection.
    ///
    /// # Errors
    /// This function errs if opening a `WebSocket` connection to URL provided by `signaling_server_url` fails.
    pub fn new(
        signaling_server_url: &str,
        session_id: SessionId,
        connection_type: &ConnectionType,
    ) -> crate::Result<Self> {
        let peer_connection = create_peer_connection(connection_type)?;

        let websocket = WebSocket::new(signaling_server_url).map_err(|err| {
            anyhow!(
                "failed to create connection with signaling server on {}: {:?}",
                signaling_server_url,
                err
            )
        })?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        Ok(Self {
            inner: Rc::new(RefCell::new(NetworkManagerInner {
                session_id,
                websocket,
                peer_connection,
                data_channel: None,
            })),
        })
    }

    /// Second part of the setup that begins the actual connection.
    /// Requires specifying a callbacks that are guaranteed to run
    /// when the connection opens and on each message received.

    pub fn start<T: DeserializeOwned>(
        &mut self,
        on_open_callback: impl FnMut() + Clone + 'static,
        on_message_callback: impl FnMut(T) + Clone + 'static,
    ) {
        self.start_with_retransmits(
            DEFAULT_MAX_RETRANSMITS,
            on_open_callback,
            on_message_callback,
        );
    }

    pub fn start_with_retransmits<T: DeserializeOwned>(
        &mut self,
        max_retransmits: u16,
        on_open_callback: impl FnMut() + Clone + 'static,
        on_message_callback: impl FnMut(T) + Clone + 'static,
    ) {
        let NetworkManagerInner {
            websocket,
            peer_connection,
            session_id,
            ..
        } = self.inner.borrow().clone();

        let mut init = RtcDataChannelInit::new();
        init.max_retransmits(max_retransmits);
        init.ordered(false);

        let data_channel = peer_connection
            .create_data_channel_with_data_channel_dict(&session_id.to_string(), &init);
        debug!(
            "data_channel created with label: {:?}",
            data_channel.label()
        );

        set_data_channel_on_open(&data_channel, on_open_callback.clone());
        set_data_channel_on_error(&data_channel);
        set_data_channel_on_message(&data_channel, on_message_callback.clone());

        self.inner.borrow_mut().data_channel = Some(data_channel);
        set_peer_connection_on_data_channel(
            &peer_connection,
            self.clone(),
            on_open_callback,
            on_message_callback,
        );

        set_peer_connection_on_ice_candidate(&peer_connection, websocket.clone(), session_id);
        set_peer_connection_on_ice_connection_state_change(&peer_connection);
        set_peer_connection_on_ice_gathering_state_change(&peer_connection);
        set_peer_connection_on_negotiation_needed(&peer_connection);
        set_websocket_on_open(&websocket, session_id);
        set_websocket_on_message(&websocket, peer_connection);
    }

    fn datachannel(&self) -> crate::Result<RtcDataChannel> {
        Ok(self
            .inner
            .borrow()
            .data_channel
            .as_ref()
            .ok_or_else(|| anyhow!("no data channel set on instance yet"))?
            .clone())
    }

    /// Send message to the other end of the connection.
    ///
    /// # Errors
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error:
    /// - if sending of the message was tried before data channel was established or,
    /// - if sending of the message failed.
    pub fn send_message<T: Serialize + ?Sized>(&self, message: &T) -> crate::Result<()> {
        let message = rmp_serde::to_vec(message)?;
        self.datachannel()?
            .send_with_u8_array(&message)
            .map_err(|err| anyhow!("failed to send string: {:?}", err))
    }
}
