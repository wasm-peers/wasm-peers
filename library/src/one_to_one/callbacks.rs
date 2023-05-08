use js_sys::Uint8Array;
use log::{debug, error, info};
use serde::de::DeserializeOwned;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_peers_protocol::one_to_one::SignalMessage;
use wasm_peers_protocol::{IceCandidate, SessionId};
use web_sys::{
    MessageEvent, RtcDataChannel, RtcDataChannelEvent, RtcPeerConnection,
    RtcPeerConnectionIceEvent, WebSocket,
};

use crate::one_to_one::{websocket_handler, NetworkManager};

/// also calls:
/// * `set_data_channel_on_open`
/// * `set_data_channel_on_message`
/// * `set_data_channel_on_error`
pub fn set_peer_connection_on_data_channel<T: DeserializeOwned>(
    peer_connection: &RtcPeerConnection,
    network_manager: NetworkManager,
    on_open_callback: impl FnMut() + Clone + 'static,
    on_message_callback: impl FnMut(T) + Clone + 'static,
) {
    let on_datachannel: Box<dyn FnMut(RtcDataChannelEvent)> =
        Box::new(move |data_channel_event: RtcDataChannelEvent| {
            info!("received data channel");
            let data_channel = data_channel_event.channel();

            set_data_channel_on_open(&data_channel, on_open_callback.clone());
            set_data_channel_on_error(&data_channel);
            set_data_channel_on_message(&data_channel, on_message_callback.clone());

            network_manager.inner.borrow_mut().data_channel = Some(data_channel);
        });
    let on_datachannel = Closure::wrap(on_datachannel);
    peer_connection.set_ondatachannel(Some(on_datachannel.as_ref().unchecked_ref()));
    on_datachannel.forget();
}

/// handle message sent by signaling server
pub fn set_websocket_on_message(websocket: &WebSocket, peer_connection: RtcPeerConnection) {
    {
        let on_message_callback = {
            let websocket = websocket.clone();
            let peer_connection = peer_connection;
            let on_message_callback: Box<dyn FnMut(MessageEvent)> =
                Box::new(move |ev: MessageEvent| {
                    let Ok(message) = ev.data().dyn_into::<Uint8Array>().map(|v| v.to_vec()) else {
                        error!("failed to convert message to Uint8Array");
                        return;
                    };
                    let Ok(message) = rmp_serde::from_slice(message.as_slice()) else {
                        error!("failed to deserialize message");
                        return;
                    };
                    let websocket_clone = websocket.clone();
                    let peer_connection_clone = peer_connection.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        if let Err(err) = websocket_handler::handle_websocket_message(
                            message,
                            peer_connection_clone,
                            websocket_clone,
                        )
                        .await
                        {
                            error!("error handling websocket message: {:?}", err);
                        }
                    });
                });
            Closure::wrap(on_message_callback)
        };
        websocket.set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));
        on_message_callback.forget();
    }
}

/// once web socket is open, send a request to start or join a session
pub fn set_websocket_on_open(websocket: &WebSocket, session_id: SessionId) {
    {
        let websocket_clone = websocket.clone();
        let on_open_callback: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
            let signal_message = SignalMessage::SessionJoin(session_id);
            let signal_message =
                rmp_serde::to_vec(&signal_message).expect("failed serializing SignalMessage");
            websocket_clone
                .send_with_u8_array(&signal_message)
                .expect("failed sending start-or-join message to the websocket");
        });
        let on_open_callback = Closure::wrap(on_open_callback);
        websocket.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));
        on_open_callback.forget();
    }
}

pub fn set_data_channel_on_message<T: DeserializeOwned>(
    data_channel: &RtcDataChannel,
    mut on_message_callback: impl FnMut(T) + 'static,
) {
    let on_message_callback: Box<dyn FnMut(MessageEvent)> = Box::new(move |ev: MessageEvent| {
        let message = ev.data().dyn_into::<Uint8Array>().ok();
        if let Some(message) = message.and_then(|t| rmp_serde::from_slice(&t.to_vec()).ok()) {
            debug!("message from datachannel (will call on_message)");
            on_message_callback(message);
        }
    });
    let on_message_callback = Closure::wrap(on_message_callback);
    data_channel.set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));
    on_message_callback.forget();
}

pub fn set_data_channel_on_error(data_channel: &RtcDataChannel) {
    let on_error: Box<dyn FnMut(JsValue)> = Box::new(move |data_channel_error| {
        error!("data channel error: {:?}", data_channel_error);
    });
    let on_error = Closure::wrap(on_error);
    data_channel.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();
}

pub fn set_data_channel_on_open(
    data_channel: &RtcDataChannel,
    mut on_open_callback: impl FnMut() + 'static,
) {
    let on_open_callback: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
        debug!("data channel is now open, calling on_open!");
        on_open_callback();
    });
    let on_open_callback = Closure::wrap(on_open_callback);
    data_channel.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));
    on_open_callback.forget();
}

pub fn set_peer_connection_on_ice_connection_state_change(peer_connection: &RtcPeerConnection) {
    let peer_connection_clone = peer_connection.clone();
    let on_ice_connection_state_change: Box<dyn FnMut()> = Box::new(move || {
        debug!(
            "connection state change: {:?}",
            peer_connection_clone.ice_connection_state()
        );
    });
    let on_ice_connection_state_change = Closure::wrap(on_ice_connection_state_change);
    peer_connection.set_oniceconnectionstatechange(Some(
        on_ice_connection_state_change.as_ref().unchecked_ref(),
    ));
    on_ice_connection_state_change.forget();
}

pub fn set_peer_connection_on_ice_candidate(
    peer_connection: &RtcPeerConnection,
    websocket: WebSocket,
    session_id: SessionId,
) {
    let on_ice_candidate: Box<dyn FnMut(RtcPeerConnectionIceEvent)> =
        Box::new(move |ev: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = ev.candidate() {
                let signaled_candidate = IceCandidate {
                    candidate: candidate.candidate(),
                    sdp_mid: candidate.sdp_mid(),
                    sdp_m_line_index: candidate.sdp_m_line_index(),
                };
                debug!("signaled candidate: {:#?}", signaled_candidate);

                let signal_message = SignalMessage::IceCandidate(session_id, signaled_candidate);
                let signal_message =
                    rmp_serde::to_vec(&signal_message).expect("failed to serialize SignalMessage");

                websocket
                    .send_with_u8_array(&signal_message)
                    .unwrap_or_else(|_| error!("failed to send one of the ICE candidates"));
            }
        });
    let on_ice_candidate = Closure::wrap(on_ice_candidate);
    peer_connection.set_onicecandidate(Some(on_ice_candidate.as_ref().unchecked_ref()));
    on_ice_candidate.forget();
}
