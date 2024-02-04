/*!
Library module for implementation of the many-to-many topology of peer communication.

Each peer in session is an equal, with ability to send and receive messages from any other peer.
Unlike with one-to-many topology, any peer can leave at any time without compromising the network.

To identify peers you should store [`UserId`] accessible inside `on_open_callback` in some custom structure.
Then you can use it in [`NetworkManager::send_message`] to specify exactly which peer should receive the message.

# Example

In this example we create 3 peers that all establish connection with each other.
Each of the peers will send a `ping` message to each new connection.
Also each peer will respond with a `pong` response.
Overall we will expect 6 `ping` and 6 `pong` messages (3 connections, both peers in each).
```
use wasm_peers::many_to_many::NetworkManager;
use wasm_peers::{ConnectionType, SessionId};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::console;

// there should be a signaling server from accompanying crate listening on this port
const SIGNALING_SERVER_URL: &str = "ws://0.0.0.0:9001/one-to-many";
const STUN_SERVER_URL: &str = "stun:openrelay.metered.ca:80";

let peer_generator = || {
    let mut peer = NetworkManager::new(
        SIGNALING_SERVER_URL,
        SessionId::new(1),
        ConnectionType::Stun { urls: STUN_SERVER_URL.to_string() },
    )
    .expect("failed to connect to signaling server");

    let peer_on_open = {
        let peer = peer.clone();
        move |user_id| {
            console::log_1(&format!("connection to peer established: {:?}", user_id).into());
            if let Err(err) = peer.send_message(user_id, "ping!") {
                console::log_1(&format!("failed to send message: {:?}", err).into());
            }
        }
    };

    let peer_on_message = {
        let peer = peer.clone();
        move |user_id, message: String| {
            console::log_1(
                &format!(
                    "peer received message from other peer {:?}: {}",
                    user_id, message
                )
                .into(),
            );
            if let Err(err) = peer.send_message(user_id, &"pong!".to_owned()) {
                console::log_1(&format!("failed to send message: {:?}", err).into());
            }
        }
    };
    peer.start(peer_on_open, peer_on_message);
};
peer_generator();
peer_generator();
peer_generator();
```
 */

use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_peers_protocol::{SessionId, UserId};

use crate::one_to_many::NetworkManager as OneToManyNetworkManager;
use crate::ConnectionType;

/// Abstraction over `WebRTC` peer-to-peer connection.
/// Structure representing equal peer in many-to-many topology.
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
    inner: OneToManyNetworkManager,
}

impl NetworkManager {
    /// Creates an instance with all resources required to create a connections to other peers.
    /// Requires an  address of an signaling server instance,
    /// session id by which it will identify connecting other peers and type of connection.
    ///
    /// # Errors
    /// This function errs if opening a `WebSocket` connection to URL provided by `signaling_server_url` fails.
    pub fn new(
        signaling_server_url: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> crate::Result<Self> {
        Ok(Self {
            inner: OneToManyNetworkManager::new(
                signaling_server_url,
                session_id,
                connection_type,
                true,
            )?,
        })
    }

    /// Second part of the setup that begins the actual connection.
    /// Requires specifying a callbacks that are guaranteed to run
    /// when a new connection opens and on each message received.
    /// It takes [`UserId`] as an argument which helps identify sending peer.
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

    /// Sends message over established data channel to a single peer represented by
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

    /// Send message to a all connected client-users.
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
