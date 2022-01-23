use crate::one_to_many::NetworkManager as OneToManyNetworkManager;
use crate::ConnectionType;
use rusty_games_protocol::{SessionId, UserId};
use wasm_bindgen::JsValue;

#[derive(Debug, Clone)]
pub struct NetworkManager {
    inner: OneToManyNetworkManager,
}

impl NetworkManager {
    /// Creates an instance with all resources required to create a connections to client-peers.
    /// Requires an IP address of an signaling server instance,
    /// session id by which it will identify connecting pair of peers and type of connection.
    pub fn new(
        ws_ip_address: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        Ok(NetworkManager {
            inner: OneToManyNetworkManager::new(ws_ip_address, session_id, connection_type, true)?,
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
