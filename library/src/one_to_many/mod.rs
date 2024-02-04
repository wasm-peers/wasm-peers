/*!
Library module for the one-to-many topology in client-server architecture.
There can be exactly one instance of [`MiniServer`] and arbitrary number of [`MiniClient`]'s
connected to the same session.

A [`RtcPeerConnection`] with an accompanying [`RtcDataChannel`] will be established between the [`MiniServer`]
and each of the [`MiniClient`]'s.
[`MiniServer`] can decide whether to send a message to a single peer,
identified by [`UserId`] returned by signaling server during connection establishment method,
with [`MiniServer::send_message`], or to fire to all clients with [`MiniServer::send_message_to_all`].

[`MiniClient`] only has an option to message the host with [`MiniClient::send_message_to_host`].

# Example

This example shows three peers connecting, with one being a dedicated host.
Host waits for both peers to connect and only then sends `ping` messages to both
and clients independently respond with `pong` messages.

```
use wasm_peers::one_to_many::{MiniClient, MiniServer};
use wasm_peers::ConnectionType;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_peers_protocol::SessionId;
use web_sys::console;

const SIGNALING_SERVER_URL: &str = "ws://0.0.0.0:9001/one-to-many";
const STUN_SERVER_URL: &str = "stun:openrelay.metered.ca:80";

let mut server = MiniServer::new(
    SIGNALING_SERVER_URL,
    SessionId::new(1),
    ConnectionType::Stun { urls: STUN_SERVER_URL.to_string() },
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
    move |user_id, message: &str| {
        console::log_1(
            &format!(
                "server received message from client {:?}: {}",
                user_id, message
            )
            .into(),
        );
    }
};
server.start(server_on_open, server_on_message);

let client_generator = || {
    let mut client = MiniClient::new(
        SIGNALING_SERVER_URL,
        SessionId::new(1),
        ConnectionType::Stun { urls: STUN_SERVER_URL.to_string() },
    )
    .unwrap();
    let client_on_open = || { /* do nothing */ };
    let client_clone = client.clone();
    let client_on_message = {
        move |message| {
            console::log_1(&format!("client received message: {}", message).into());
            client_clone.send_message_to_host("pong!").unwrap();
        }
    };
    client.start(client_on_open, client_on_message);
};
client_generator();
client_generator();
```
*/

mod callbacks;
mod websocket_handler;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use anyhow::anyhow;
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_peers_protocol::{SessionId, UserId};
use web_sys::{RtcDataChannel, RtcPeerConnection, WebSocket};

use crate::constants::DEFAULT_MAX_RETRANSMITS;
use crate::one_to_many::callbacks::{set_websocket_on_message, set_websocket_on_open};
use crate::ConnectionType;

#[derive(Debug, Clone)]
struct Connection {
    peer_connection: RtcPeerConnection,
    data_channel: Option<RtcDataChannel>,
}

impl Connection {
    const fn new(peer_connection: RtcPeerConnection, data_channel: Option<RtcDataChannel>) -> Self {
        Self {
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
pub struct NetworkManager {
    inner: Rc<RefCell<NetworkManagerInner>>,
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
        connection_type: ConnectionType,
        is_host: bool,
    ) -> crate::Result<Self> {
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
                connection_type,
                is_host,
                connections: HashMap::new(),
            })),
        })
    }

    pub fn start<T: DeserializeOwned>(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
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
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
    ) {
        let websocket = self.inner.borrow().websocket.clone();
        let session_id = self.inner.borrow().session_id;
        let is_host = self.inner.borrow().is_host;

        set_websocket_on_open(&websocket, session_id, is_host);
        set_websocket_on_message(
            &websocket,
            self.clone(),
            max_retransmits,
            on_open_callback,
            on_message_callback,
            is_host,
        );
    }

    /// Send message to a connected client-user identified by unique [`UserId`]
    ///
    /// # Errors
    /// This function can err if:
    /// - sending of the message was tried before data channel was established or,
    /// - sending of the message failed.
    pub fn send_message<T: Serialize + ?Sized>(
        &self,
        user_id: UserId,
        message: &T,
    ) -> crate::Result<()> {
        let message = rmp_serde::to_vec(message)?;
        self.inner
            .borrow()
            .connections
            .get(&user_id)
            .ok_or_else(|| anyhow!("no connection for user {}", user_id))?
            .data_channel
            .as_ref()
            .ok_or_else(|| anyhow!("no data channel setup yet for user {}", user_id))?
            .send_with_u8_array(&message)
            .map_err(|err| anyhow!("failed to send string: {:?}", err))
    }

    /// Send message to a all connected client-users.
    ///
    /// # Errors
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error:
    /// - if sending of the message was tried before data channel was established or,
    /// - if sending of the message failed.
    pub fn send_message_to_all<T: Serialize + ?Sized>(&self, message: &T) -> crate::Result<()> {
        let message = rmp_serde::to_vec(message)?;
        for data_channel in self
            .inner
            .borrow()
            .connections
            .values()
            .filter_map(|connection| connection.data_channel.as_ref())
        {
            // TODO(tkarwowski): some may fail, should we return a list results?
            let _result = data_channel.send_with_u8_array(&message);
        }
        Ok(())
    }
}

/// Abstraction over `WebRTC` peer-to-peer connection.
/// Structure representing server in client-server topology.
///
/// `WebRTC` data channel communication abstracted to a single class.
/// All setup is handled internally, you must only provide callbacks
/// for when the connection opens and for handling incoming messages.
/// It also provides a method of sending data to the other end of the connection.
///
/// Only works with [wasm-peers-signaling-server](https://docs.rs/wasm-peers-signaling-server/latest/wasm_peers_signaling_server/) instance,
/// whose full  address must be provided.
///
/// Start-up flow is divided into two methods [`MiniServer::new`] and [`MiniServer::start`]
/// to allow possibility of referring to network manger itself from the callbacks.
///
/// This class is a  pointer to the underlying resource and can be cloned freely.
#[derive(Debug, Clone)]
pub struct MiniServer {
    inner: NetworkManager,
}

impl MiniServer {
    /// Creates an instance with all resources required to create a connections to client-peers.
    /// Requires an  address of an signaling server instance,
    /// session id by which it will identify connecting pair of peers and type of connection.
    ///
    /// # Errors
    /// This function errs if opening a `WebSocket` connection to URL provided by `signaling_server_url` fails.
    pub fn new(
        signaling_server_url: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: NetworkManager::new(signaling_server_url, session_id, connection_type, true)?,
        })
    }

    /// Second part of the setup that begins the actual connection.
    /// Requires specifying a callbacks that are guaranteed to run
    /// when the connection opens and on each message received.
    /// It takes [`UserId`] as an argument which helps identify which client-peer.
    pub fn start<T: DeserializeOwned>(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
    ) {
        self.inner.start(on_open_callback, on_message_callback);
    }

    pub fn start_with_retransmits<T: DeserializeOwned>(
        &mut self,
        max_retransmits: u16,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
    ) {
        self.inner
            .start_with_retransmits(max_retransmits, on_open_callback, on_message_callback);
    }

    /// Sends message over established data channel with a single client-peer represented by
    /// the [`UserId`] returned by signaling server during connection establishment.
    ///
    /// # Errors
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error:
    /// - if sending of the message was tried before data channel was established or,
    /// - if sending of the message failed.
    pub fn send_message<T: Serialize + ?Sized>(
        &self,
        user_id: UserId,
        message: &T,
    ) -> crate::Result<()> {
        self.inner.send_message(user_id, message)
    }

    /// Convenience function that sends the same message to all connected client-peers.
    ///
    /// # Errors
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error:
    /// - if sending of the message was tried before data channel was established or,
    /// - if sending of the message failed.
    pub fn send_message_to_all<T: Serialize + ?Sized>(&self, message: &T) -> crate::Result<()> {
        self.inner.send_message_to_all(message)
    }
}

/// Abstraction over `WebRTC` peer-to-peer connection.
/// Same as [`MiniServer`], but representing clients in client-server topology.
#[derive(Debug, Clone)]
pub struct MiniClient {
    inner: NetworkManager,
}

impl MiniClient {
    /// Same as [`MiniServer::new`]
    ///
    /// # Errors
    /// This function errs if opening a `WebSocket` connection to URL provided by `signaling_server_url` fails.
    pub fn new(
        signaling_server_url: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: NetworkManager::new(signaling_server_url, session_id, connection_type, false)?,
        })
    }

    pub fn start<T: DeserializeOwned>(
        &mut self,
        mut on_open_callback: impl FnMut() + Clone + 'static,
        mut on_message_callback: impl FnMut(T) + Clone + 'static,
    ) {
        let on_open_callback = move |_| on_open_callback();
        let on_message_callback = move |_, message| on_message_callback(message);
        self.inner.start(on_open_callback, on_message_callback);
    }

    /// Same as [`MiniServer::start`], but callbacks don't take `UserId` argument, as it will always be host.
    pub fn start_with_retransmits<T: DeserializeOwned>(
        &mut self,
        max_retransmits: u16,
        mut on_open_callback: impl FnMut() + Clone + 'static,
        mut on_message_callback: impl FnMut(T) + Clone + 'static,
    ) {
        let on_open_callback = move |_| on_open_callback();
        let on_message_callback = move |_, message| on_message_callback(message);
        self.inner
            .start_with_retransmits(max_retransmits, on_open_callback, on_message_callback);
    }

    /// Way of communicating with peer-server
    /// Send message to the other end of the connection.
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error.
    ///
    /// # Errors
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_open_callback` triggers.
    /// Otherwise it will result in an error:
    /// - if sending of the message was tried before data channel was established or,
    /// - if sending of the message failed.
    pub fn send_message_to_host<T: Serialize + ?Sized>(&self, message: &T) -> crate::Result<()> {
        self.inner.send_message_to_all(message)
    }
}
