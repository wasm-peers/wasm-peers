mod callbacks;
mod websocket_handler;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::one_to_many::callbacks::{set_websocket_on_message, set_websocket_on_open};
use crate::ConnectionType;
use rusty_games_protocol::{SessionId, UserId};
use std::rc::Rc;
use log::debug;
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
            .ok_or_else(|| JsValue::from_str(&format!("no connection for user {}", user_id.inner)))?
            .data_channel
            .as_ref()
            .ok_or_else(|| {
                JsValue::from_str(&format!(
                    "no data channel setup yet for user {}",
                    user_id.inner
                ))
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

#[derive(Debug, Clone)]
pub struct MiniServer {
    inner: NetworkManager,
}

impl MiniServer {
    pub fn new(
        ws_ip_address: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        Ok(MiniServer {
            inner: NetworkManager::new(ws_ip_address, session_id, connection_type, true)?,
        })
    }

    pub fn start(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        self.inner.start(on_open_callback, on_message_callback)
    }

    pub fn send_message(&self, user_id: UserId, message: &str) -> Result<(), JsValue> {
        self.inner.send_message(user_id, message)
    }

    pub fn send_message_to_all(&self, message: &str) {
        self.inner.send_message_to_all(message)
    }
}

#[derive(Debug, Clone)]
pub struct MiniClient {
    inner: NetworkManager,
}

impl MiniClient {
    pub fn new(
        ws_ip_address: &str,
        session_id: SessionId,
        connection_type: ConnectionType,
    ) -> Result<Self, JsValue> {
        Ok(MiniClient {
            inner: NetworkManager::new(ws_ip_address, session_id, connection_type, false)?,
        })
    }

    pub fn start(
        &mut self,
        on_open_callback: impl FnMut(UserId) + Clone + 'static,
        on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    ) -> Result<(), JsValue> {
        self.inner.start(on_open_callback, on_message_callback)
    }

    pub fn send_message_to_host(&self, message: &str) -> Result<(), JsValue> {
        self.inner.send_message_to_all(message);
        // TODO: we always return success, but this is subject to change
        Ok(())
    }
}
