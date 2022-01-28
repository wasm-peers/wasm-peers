/*!
Library module for implementation of the many-to-many topology of peer communication.

Each peer in session is an equal, with ability to send and receive messages from any other peer.
Unlike with one-to-many topology, any peer can leave at any time without compromising the network.

To identify peers you should store [UserId] accessible inside `on_open_callback` in some custom structure.
Then you can use it in [NetworkManager::send_message] to specify exactly which peer should receive the message.

# Example

In this example we create 3 peers that all establish connection with each other.
Each of the peers will send a `ping` message to each new connection.
Also each peer will respond with a `pong` response.
Overall we will expect 6 `ping` and 6 `pong` messages (3 connections, both peers in each).
```

use rusty_games_library::many_to_many::NetworkManager;
use rusty_games_library::{ConnectionType, SessionId};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::console;

// there should be a signaling server from accompanying crate listening on this port
const SIGNALING_SERVER_URL: &str = "ws://0.0.0.0:9001/one-to-many";

let opened_connections_count = Rc::new(RefCell::new(0));
let received_messages_count = Rc::new(RefCell::new(0));

let peer_generator = || {
    let mut server = NetworkManager::new(
        SIGNALING_SERVER_URL,
        SessionId::new("dummy-session-id".to_string()),
        ConnectionType::Stun,
    )
    .unwrap();

    let server_clone = server.clone();
    let opened_connections_count = opened_connections_count.clone();
    let server_on_open = {
        move |user_id| {
            console::log_1(&format!("connection to user established: {:?}", user_id).into());
            *opened_connections_count.borrow_mut() += 1;
            server_clone.send_message(user_id, "ping!");
        }
    };

    let server_clone = server.clone();
    let received_messages_count = received_messages_count.clone();
    let server_on_message = {
        move |user_id, message| {
            console::log_1(
                &format!(
                    "server received message from client {:?}: {}",
                    user_id, message
                )
                .into(),
            );
            *received_messages_count.borrow_mut() += 1;
            server_clone.send_message(user_id, "pong!");
        }
    };
    server.start(server_on_open, server_on_message).unwrap();
};
peer_generator();
peer_generator();
peer_generator();
```
 */

use crate::one_to_many::NetworkManager as OneToManyNetworkManager;
use crate::ConnectionType;
use rusty_games_protocol::{SessionId, UserId};
use wasm_bindgen::JsValue;

/// Abstraction over WebRTC peer-to-peer connection.
/// Structure representing equal peer in many-to-many topology.
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
    inner: OneToManyNetworkManager,
}

impl NetworkManager {
    /// Creates an instance with all resources required to create a connections to other peers.
    /// Requires an IP address of an signaling server instance,
    /// session id by which it will identify connecting other peers and type of connection.
    pub fn new(
        signaling_server_url: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        Ok(NetworkManager {
            inner: OneToManyNetworkManager::new(signaling_server_url, session_id, connection_type, true)?,
        })
    }

    /// Second part of the setup that begins the actual connection.
    /// Requires specifying a callbacks that are guaranteed to run
    /// when a new connection opens and on each message received.
    /// It takes [UserId] as an argument which helps identify sending peer.
    pub fn start(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        self.inner.start(on_open_callback, on_message_callback)
    }

    /// Sends message over established data channel to a single peer represented by
    /// the [UserId] returned by signaling server during connection establishment.
    pub fn send_message(&self, user_id: UserId, message: &str) -> Result<(), JsValue> {
        self.inner.send_message(user_id, message)
    }

    /// Convenience method that sends the same message to all connected peers.
    pub fn send_message_to_all(&self, message: &str) {
        self.inner.send_message_to_all(message)
    }
}
