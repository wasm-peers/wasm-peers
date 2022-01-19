mod callbacks;
mod websocket_handler;

use std::cell::RefCell;
use std::collections::HashMap;

use crate::one_to_many::callbacks::{set_websocket_on_message, set_websocket_on_open};
use crate::ConnectionType;
use log::debug;
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
pub struct NetworkManager {
    inner: Rc<RefCell<NetworkManagerInner>>,
}

impl NetworkManager {
    pub fn new(
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

    pub fn start(
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

    /// Send message to the other end of the connection.
    /// It might fail if the connection is not yet set up
    /// and thus should only be called after `on_message_callback` triggers.
    /// Otherwise it will result in an error.
    pub fn send_message_to_all(&self, message: &str) {
        for data_channel in self
            .inner
            .borrow()
            .connections
            .values()
            .filter_map(|connection| connection.data_channel.as_ref())
        {
            data_channel.send_with_str(message);
        }
    }
}
