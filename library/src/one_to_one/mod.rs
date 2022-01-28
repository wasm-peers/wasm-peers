/*!
Library module for one-to-one network topology in simple tunnel connection.

After connection is established both peers are treated equally and have an opportunity to send messages
with [NetworkManager::send_message] method.

# Example

This example shows two peers sending `ping` and `pong` messages to each other.

```
use rusty_games_library::ConnectionType;
use rusty_games_library::one_to_one::NetworkManager;
use web_sys::console;

const SIGNALING_SERVER_URL: &str = "ws://0.0.0.0:9001/one-to-one";

let session_id = SessionId::new("some-session-id".to_string());
let mut server = NetworkManager::new(
    SIGNALING_SERVER_URL,
    session_id.clone(),
    ConnectionType::Stun,
)
.unwrap();

let server_clone = server.clone();
let server_on_open = move || server_clone.send_message("ping!").unwrap();
let server_on_message = {
    let server_received_message = server_received_message.clone();
    move |message| {
        console::log_1(&format!("server received message: {}", message).into());
        *server_received_message.borrow_mut() = true;
    }
};
server.start(server_on_open, server_on_message).unwrap();

let mut client = NetworkManager::new(
    SIGNALING_SERVER_URL,
    session_id,
    ConnectionType::Stun,
)
.unwrap();
let client_on_open = || { /* do nothing */ };
let client_clone = client.clone();
let client_on_message = {
    let client_received_message = client_received_message.clone();
    move |message| {
        console::log_1(&format!("client received message: {}", message).into());
        client_clone.send_message("pong!").unwrap();
        *client_received_message.borrow_mut() = true;
    }
};
client.start(client_on_open, client_on_message).unwrap();
```
*/

use crate::one_to_one::callbacks::{
    set_data_channel_on_error, set_data_channel_on_message, set_data_channel_on_open,
    set_peer_connection_on_data_channel, set_peer_connection_on_ice_candidate,
    set_peer_connection_on_ice_connection_state_change,
    set_peer_connection_on_ice_gathering_state_change, set_peer_connection_on_negotiation_needed,
    set_websocket_on_message, set_websocket_on_open,
};
use crate::utils::{create_peer_connection, ConnectionType};
use log::debug;
use rusty_games_protocol::SessionId;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys::RtcPeerConnection;
use web_sys::{RtcDataChannel, WebSocket};

mod callbacks;
mod websocket_handler;

#[derive(Debug, Clone)]
pub(crate) struct NetworkManagerInner {
    session_id: SessionId,
    websocket: WebSocket,
    peer_connection: RtcPeerConnection,
    pub(crate) data_channel: Option<RtcDataChannel>,
}

/// Abstraction over WebRTC peer-to-peer connection.
/// Structure representing one of two equal peers.
///
/// WebRTC data channel communication abstracted to a single class.
/// All setup is handled internally, you must only provide callbacks
/// for when the connection opens and for handling incoming messages.
/// It also provides a method of sending data to the other end of the connection.
///
/// Only works with [rusty-games-signaling-server](../../rusty_games_signaling_server/index.html) instance,
/// whose full IP address must be provided.
///
/// Startup flow is divided into two methods [NetworkManager::new] and [NetworkManager::start]
/// to allow possibility of referring to network manger itself from the callbacks.
///
/// This class is a cloneable pointer to the underlying resource and can be cloned freely.
#[derive(Debug, Clone)]
pub struct NetworkManager {
    pub(crate) inner: Rc<RefCell<NetworkManagerInner>>,
}

impl NetworkManager {
    /// Creates an instance with all resources required to create a connection.
    /// Requires an IP address of an signaling server instance,
    /// session id by which it will identify connecting pair of peers and type of connection.
    pub fn new(
        signaling_server_url: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        let peer_connection = create_peer_connection(&connection_type)?;

        let websocket = WebSocket::new(signaling_server_url)?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        Ok(NetworkManager {
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
    pub fn start(
        &mut self,
        on_open_callback: impl FnMut() + Clone + 'static,
        on_message_callback: impl FnMut(String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        let NetworkManagerInner {
            websocket,
            peer_connection,
            session_id,
            ..
        } = self.inner.borrow().clone();

        let data_channel = peer_connection.create_data_channel(&session_id.clone().into_inner());
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

        set_peer_connection_on_ice_candidate(
            &peer_connection,
            websocket.clone(),
            session_id.clone(),
        );
        set_peer_connection_on_ice_connection_state_change(&peer_connection);
        set_peer_connection_on_ice_gathering_state_change(&peer_connection);
        set_peer_connection_on_negotiation_needed(&peer_connection);
        set_websocket_on_open(&websocket, session_id);
        set_websocket_on_message(&websocket, peer_connection);

        Ok(())
    }

    /// Send message to the other end of the connection.
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error.
    pub fn send_message(&self, message: &str) -> Result<(), JsValue> {
        debug!("server will try to send a message: {:?}", &message);
        self.inner
            .borrow()
            .data_channel
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no data channel set on instance yet"))?
            // this is an ugly fix to the fact, that if you send empty string as message
            // webrtc fails with a cryptic "The operation failed for an operation-specific reason"
            // message
            .send_with_str(&format!("x{}", message))
    }
}
