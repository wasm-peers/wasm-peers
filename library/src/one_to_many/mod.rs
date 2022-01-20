/*!
Library module for one-to-many network topology in client-server architecture.
There must be exactly one instance of [MiniServer] and arbitrary number of [MiniClient]'s
connected to the same session.

A RtcPeerConnection with an accompanying RtcDataChannel will be established between the [MiniServer]
and each of the [MiniClient]'s. [MiniServer] can decide whether to send a message to a single peer,
identified by [UserId] returned by signaling server during connection establishment with [MiniServer::send_message]
method, or to fire to all clients with [MiniServer::send_message_to_all].
[MiniClient] only has an option to message the host with [MiniClient::send_message_to_host].

# Example

This example shows three peers connecting, with one being a dedicated host.
Host waits for both peers to connect and then sends `ping` messages to both
and they independently respond with `pong` messages.

```
use rusty_games_library::one_to_many::{MiniClient, MiniServer};
use rusty_games_library::ConnectionType;
use std::cell::RefCell;
use std::rc::Rc;
use rusty_games_protocol::SessionId;
use web_sys::console;

const WS_IP_ADDRESS: &str = "ws://0.0.0.0:9001/one-to-many";

let mut server = MiniServer::new(
    WS_IP_ADDRESS,
    SessionId::new("dummy-session-id".to_string()),
    ConnectionType::Stun,
)
.unwrap();
let server_open_connections_count = Rc::new(RefCell::new(0));

let server_clone = server.clone();
let server_on_open = {
    let server_open_connections_count = server_open_connections_count.clone();
    move |user_id| {
        console::log_1(&format!("connection to user established: {:?}", user_id).into());
        *server_open_connections_count.borrow_mut() += 1;
        if *server_open_connections_count.borrow() == 2 {
            server_clone.send_message_to_all("ping!");
        }
    }
};
let server_on_message = {
    let server_received_message = server_received_message.clone();
    move |user_id, message| {
        console::log_1(
            &format!(
                "server received message from client {:?}: {}",
                user_id, message
            )
            .into(),
        );
        *server_received_message.borrow_mut() = true;
    }
};
server.start(server_on_open, server_on_message).unwrap();

let client_generator = || {
    let mut client = MiniClient::new(
        WS_IP_ADDRESS,
        SessionId::new("dummy-session-id".to_string()),
        ConnectionType::Stun,
    )
    .unwrap();
    let client_on_open = |_| { /* do nothing */ };
    let client_clone = client.clone();
    let client_on_message = {
        let client_received_message = client_received_message.clone();
        move |_, message| {
            console::log_1(&format!("client received message: {}", message).into());
            client_clone.send_message_to_host("pong!");
            *client_received_message.borrow_mut() = true;
        }
    };
    client.start(client_on_open, client_on_message).unwrap();
};
client_generator();
client_generator();
```
*/

mod callbacks;
mod websocket_handler;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::one_to_many::callbacks::{set_websocket_on_message, set_websocket_on_open};
use crate::ConnectionType;
use rusty_games_protocol::{SessionId, UserId};
use std::rc::Rc;
use wasm_bindgen::JsValue;
use web_sys::{RtcDataChannel, RtcPeerConnection, WebSocket};

#[derive(Debug, Clone)]
struct Connection {
    peer_connection: RtcPeerConnection,
    data_channel: Option<RtcDataChannel>,
}

impl Connection {
    fn new(peer_connection: RtcPeerConnection, data_channel: Option<RtcDataChannel>) -> Self {
        Connection {
            peer_connection,
            data_channel,
        }
    }
}

#[derive(Debug)]
struct NetworkManagerInner {
    session_id: SessionId,
    websocket: WebSocket,
    connection_type: ConnectionType,
    is_host: bool,
    connections: HashMap<UserId, Connection>,
}
#[derive(Debug, Clone)]
pub(crate) struct NetworkManager {
    inner: Rc<RefCell<NetworkManagerInner>>,
}

impl NetworkManager {
    fn new(
        ws_ip_address: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
        is_host: bool,
    ) -> Result<Self, JsValue> {
        let websocket = WebSocket::new(ws_ip_address)?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        Ok(NetworkManager {
            inner: Rc::new(RefCell::new(NetworkManagerInner {
                session_id,
                websocket,
                connection_type,
                is_host,
                connections: HashMap::new(),
            })),
        })
    }

    fn start(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        let websocket = self.inner.borrow().websocket.clone();
        let session_id = self.inner.borrow().session_id.clone();
        let is_host = self.inner.borrow().is_host;

        set_websocket_on_open(&websocket, session_id, is_host);
        set_websocket_on_message(
            &websocket,
            self.clone(),
            on_open_callback,
            on_message_callback,
            is_host,
        );

        Ok(())
    }

    fn send_message(&self, user_id: UserId, message: &str) -> Result<(), JsValue> {
        self.inner
            .borrow()
            .connections
            .get(&user_id)
            .ok_or_else(|| JsValue::from_str(&format!("no connection for user {}", user_id)))?
            .data_channel
            .as_ref()
            .ok_or_else(|| {
                JsValue::from_str(&format!("no data channel setup yet for user {}", user_id))
            })?
            // this is an ugly fix to the fact, that if you send empty string as message
            // webrtc fails with a cryptic "The operation failed for an operation-specific reason"
            // message
            .send_with_str(&format!("x{}", message))
    }

    fn send_message_to_all(&self, message: &str) {
        for data_channel in self
            .inner
            .borrow()
            .connections
            .values()
            .filter_map(|connection| connection.data_channel.as_ref())
        {
            data_channel
                // this is an ugly fix to the fact, that if you send empty string as message
                // webrtc fails with a cryptic "The operation failed for an operation-specific reason"
                // message
                .send_with_str(&format!("x{}", message))
                .expect("one of data channels is already closed");
        }
    }
}

/// Abstraction over WebRTC peer-to-peer connection.
/// Structure representing server in client-server topology.
///
/// WebRTC data channel communication abstracted to a single class.
/// All setup is handled internally, you must only provide callbacks
/// for when the connection opens and for handling incoming messages.
/// It also provides a method of sending data to the other end of the connection.
///
/// Only works with [rusty-games-signaling-server](../../rusty_games_signaling_server/index.html) instance,
/// whose full IP address must be provided.
///
/// Startup flow is divided into two methods [MiniServer::new] and [MiniServer::start]
/// to allow possibility of referring to network manger itself from the callbacks.
///
/// This class is a cloneable pointer to the underlying resource and can be cloned freely.
#[derive(Debug, Clone)]
pub struct MiniServer {
    inner: NetworkManager,
}

impl MiniServer {
    /// Creates an instance with all resources required to create a connections to client-peers.
    /// Requires an IP address of an signaling server instance,
    /// session id by which it will identify connecting pair of peers and type of connection.
    pub fn new(
        ws_ip_address: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        Ok(MiniServer {
            inner: NetworkManager::new(ws_ip_address, session_id, connection_type, true)?,
        })
    }

    /// Second part of the setup that begins the actual connection.
    /// Requires specifying a callbacks that are guaranteed to run
    /// when the connection opens and on each message received.
    /// It takes [UserId] as an argument which helps identify which client-peer.
    pub fn start(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        self.inner.start(on_open_callback, on_message_callback)
    }

    /// Sends message over established data channel with a single client-peer represented by
    /// the [UserId] returned by signaling server during connection establishment.
    pub fn send_message(&self, user_id: UserId, message: &str) -> Result<(), JsValue> {
        self.inner.send_message(user_id, message)
    }

    /// Convenience function that sends the same message to all connected client-peers.
    pub fn send_message_to_all(&self, message: &str) {
        self.inner.send_message_to_all(message)
    }
}

/// Abstraction over WebRTC peer-to-peer connection.
/// Same as [MiniServer], but representing clients in client-server topology.
#[derive(Debug, Clone)]
pub struct MiniClient {
    inner: NetworkManager,
}

impl MiniClient {
    /// Same as [MiniServer::new]
    pub fn new(
        ws_ip_address: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        Ok(MiniClient {
            inner: NetworkManager::new(ws_ip_address, session_id, connection_type, false)?,
        })
    }

    /// Same as [MiniServer::start]
    pub fn start(
        &mut self,
        // FIXME: MiniServer callbacks should take UserId as argument, it will always be host's.
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        self.inner.start(on_open_callback, on_message_callback)
    }

    /// Way of communicating with peer-server
    pub fn send_message_to_host(&self, message: &str) -> Result<(), JsValue> {
        self.inner.send_message_to_all(message);
        // TODO: we always return success, but this is subject to change
        Ok(())
    }
}
