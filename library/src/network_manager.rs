use std::rc::Rc;
use std::sync::RwLock;

use log::debug;
use wasm_bindgen::JsValue;
use web_sys::RtcPeerConnection;
use web_sys::{RtcDataChannel, WebSocket};

use rusty_games_protocol::SessionId;

use crate::callbacks::{
    set_data_channel_on_error, set_data_channel_on_message, set_data_channel_on_open,
    set_peer_connection_on_data_channel, set_peer_connection_on_ice_candidate,
    set_peer_connection_on_ice_connection_state_change,
    set_peer_connection_on_ice_gathering_state_change, set_peer_connection_on_negotiation_needed,
    set_websocket_on_message, set_websocket_on_open,
};
use crate::common::{create_stun_peer_connection, WS_IP_PORT};

pub enum ConnectionType {
    Local,
    Stun,
    StunAndTurn,
}

#[derive(Debug, Clone)]
pub(crate) struct NetworkManagerInner {
    session_id: String,
    websocket: WebSocket,
    peer_connection: RtcPeerConnection,
    pub(crate) data_channel: Option<RtcDataChannel>,
}

#[derive(Debug, Clone)]
pub struct NetworkManager {
    pub(crate) inner: Rc<RwLock<NetworkManagerInner>>,
}

impl NetworkManager {
    pub fn new(session_id: SessionId, connection_type: ConnectionType) -> Result<Self, JsValue> {
        let peer_connection = match connection_type {
            ConnectionType::Local => RtcPeerConnection::new()?,
            ConnectionType::Stun => create_stun_peer_connection()?,
            ConnectionType::StunAndTurn => unimplemented!("no turn server yet!"),
        };

        let websocket = WebSocket::new(WS_IP_PORT)?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        Ok(NetworkManager {
            inner: Rc::new(RwLock::new(NetworkManagerInner {
                session_id,
                websocket,
                peer_connection,
                data_channel: None,
            })),
        })
    }

    pub fn start(
        &mut self,
        on_open_callback: impl FnMut() + Clone + 'static,
        on_message_callback: impl FnMut(String) + Clone + 'static,
        is_host: bool,
    ) -> Result<(), JsValue> {
        let websocket = self.inner.read().unwrap().websocket.clone();
        let peer_connection = self.inner.read().unwrap().peer_connection.clone();
        let session_id = self.inner.read().unwrap().session_id.clone();

        if is_host {
            let data_channel = peer_connection.create_data_channel(&session_id);
            debug!(
                "data_channel created with label: {:?}",
                data_channel.label()
            );

            set_data_channel_on_open(&data_channel, on_open_callback);
            set_data_channel_on_error(&data_channel);
            set_data_channel_on_message(&data_channel, on_message_callback);

            self.inner.write().unwrap().data_channel = Some(data_channel);
        } else {
            set_peer_connection_on_data_channel(
                &peer_connection,
                self.clone(),
                on_open_callback,
                on_message_callback,
            );
        }

        set_peer_connection_on_ice_candidate(
            &peer_connection,
            websocket.clone(),
            session_id.clone(),
        );
        set_peer_connection_on_ice_connection_state_change(&peer_connection);
        set_peer_connection_on_ice_gathering_state_change(&peer_connection);
        set_peer_connection_on_negotiation_needed(&peer_connection);
        set_websocket_on_open(&websocket, session_id);
        set_websocket_on_message(&websocket, peer_connection, is_host);

        Ok(())
    }

    pub fn send_message(&self, message: &str) -> Result<(), JsValue> {
        debug!("server will try to send a message: {:?}", &message);
        self.inner
            .read()
            .unwrap()
            .data_channel
            .as_ref()
            .ok_or_else(|| JsValue::from_str("no data channel set on instance yet"))?
            .send_with_str(message)
    }
}
