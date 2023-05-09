use js_sys::JsString;
use log::{debug, error, info};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_peers_protocol::one_to_many::SignalMessage;
use wasm_peers_protocol::{IceCandidate, SessionId, UserId};
use web_sys::{
    MessageEvent, RtcDataChannel, RtcDataChannelEvent, RtcPeerConnection,
    RtcPeerConnectionIceEvent, WebSocket,
};

use crate::one_to_many::{websocket_handler, NetworkManager};

/// Also calls:
/// * `set_data_channel_on_open`
/// * `set_data_channel_on_message`
/// * `set_data_channel_on_error`
pub fn set_peer_connection_on_data_channel(
    peer_connection: &RtcPeerConnection,
    client_id: UserId,
    network_manager: NetworkManager,
    on_open_callback: impl FnMut(UserId) + Clone + 'static,
    on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
) {
    let on_open_callback_clone = on_open_callback;
    let on_message_callback_clone = on_message_callback;
    let on_datachannel: Box<dyn FnMut(RtcDataChannelEvent)> =
        Box::new(move |data_channel_event: RtcDataChannelEvent| {
            info!("received data channel");
            let data_channel = data_channel_event.channel();

            set_data_channel_on_open(&data_channel, client_id, on_open_callback_clone.clone());
            set_data_channel_on_error(&data_channel);
            set_data_channel_on_message(
                &data_channel,
                client_id,
                on_message_callback_clone.clone(),
            );

            if let Some(connection) = network_manager
                .inner
                .borrow_mut()
                .connections
                .get_mut(&client_id)
            {
                connection.data_channel = Some(data_channel);
            } else {
                error!("no connection for given id: {}", client_id);
            }
        });
    let on_datachannel = Closure::wrap(on_datachannel);
    peer_connection.set_ondatachannel(Some(on_datachannel.as_ref().unchecked_ref()));
    on_datachannel.forget();
}

/// handle message sent by signaling server
pub fn set_websocket_on_message(
    websocket: &WebSocket,
    network_manager: NetworkManager,
    on_open_callback: impl FnMut(UserId) + Clone + 'static,
    on_message_callback: impl FnMut(UserId, String) + Clone + 'static,
    is_host: bool,
) {
    let on_message_callback = {
        let websocket = websocket.clone();
        let on_message_callback: Box<dyn FnMut(MessageEvent)> =
            Box::new(move |ev: MessageEvent| {
                if let Ok(message) = ev.data().dyn_into::<JsString>() {
                    match serde_json_wasm::from_str(&String::from(message)) {
                        Ok(message) => {
                            let network_manager = network_manager.clone();
                            let websocket = websocket.clone();
                            let on_open_callback_clone = on_open_callback.clone();
                            let on_message_callback_clone = on_message_callback.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                if let Err(err) = websocket_handler::handle_websocket_message(
                                    network_manager,
                                    message,
                                    websocket,
                                    on_open_callback_clone,
                                    on_message_callback_clone,
                                    is_host,
                                )
                                .await
                                {
                                    error!("failed to handle websocket message: {}", err);
                                }
                            });
                        }
                        Err(err) => {
                            error!(
                                "failed to deserialize onmessage callback content: {:?}.",
                                err
                            );
                        }
                    }
                }
            });
        Closure::wrap(on_message_callback)
    };
    websocket.set_onmessage(Some(on_message_callback.as_ref().unchecked_ref()));
    on_message_callback.forget();
}

/// once web socket is open, send a request to start or join a session
pub fn set_websocket_on_open(websocket: &WebSocket, session_id: SessionId, is_host: bool) {
    let on_open_callback = {
        let websocket = websocket.clone();
        let on_open_callback: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
            let signal_message = SignalMessage::SessionJoin(session_id, is_host);
            let signal_message = serde_json_wasm::to_string(&signal_message)
                .expect("failed serializing SignalMessage");
            if let Err(err) = websocket.send_with_str(&signal_message) {
                error!("failed to send signal message: {:?}", err);
            }
        });
        Closure::wrap(on_open_callback)
    };
    websocket.set_onopen(Some(on_open_callback.as_ref().unchecked_ref()));
    on_open_callback.forget();
}

pub fn set_data_channel_on_message(
    data_channel: &RtcDataChannel,
    client_id: UserId,
    mut on_message_callback: impl FnMut(UserId, String) + 'static,
) {
    let on_message_callback: Box<dyn FnMut(MessageEvent)> = Box::new(move |ev: MessageEvent| {
        if let Some(message) = ev.data().as_string() {
            debug!(
                "message from datachannel (will call on_message): {:?}",
                message
            );
            on_message_callback(
                client_id,
                message
                    // this is an ugly fix to the fact, that if you send empty string as message
                    // webrtc fails with a cryptic "The operation failed for an operation-specific reason"
                    // message
                    .strip_prefix('x')
                    .expect("messages must have a fix-bug x prepended")
                    .to_owned(),
            );
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
    client_id: UserId,
    mut on_open_callback: impl FnMut(UserId) + 'static,
) {
    let on_open_callback: Box<dyn FnMut(JsValue)> = Box::new(move |_| {
        debug!("data channel is now open, calling on_open!");
        on_open_callback(client_id);
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
    client_id: UserId,
    websocket_clone: WebSocket,
    session_id_clone: SessionId,
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

                let signal_message =
                    SignalMessage::IceCandidate(session_id_clone, client_id, signaled_candidate);
                let signal_message = serde_json_wasm::to_string(&signal_message)
                    .expect("failed to serialize SignalMessage");

                if let Err(err) = websocket_clone.send_with_str(&signal_message) {
                    error!("failed to send one of the ICE candidates: {:?}", err);
                }
            }
        });
    let on_ice_candidate = Closure::wrap(on_ice_candidate);
    peer_connection.set_onicecandidate(Some(on_ice_candidate.as_ref().unchecked_ref()));
    on_ice_candidate.forget();
}
