use js_sys::{JsString, JSON};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use log::{error, info, warn};
use rusty_games_protocol::{SessionId, SignalMessage};
use web_sys::{console, RtcDataChannel, RtcIceConnectionState, WebSocket};
use web_sys::{MessageEvent, RtcPeerConnection, RtcPeerConnectionIceEvent};

use crate::common::{create_peer_connection, create_sdp_offer, WS_IP_PORT};

pub struct MiniServer {
    peer_connection: RtcPeerConnection,
    data_channel: RtcDataChannel,
    session_id: String,
}

impl MiniServer {
    pub fn start(session_id: String) -> Result<Rc<RefCell<Self>>, JsValue> {
        let peer_connection = create_peer_connection()?;
        info!(
            "peer connections created, states: {:?}",
            peer_connection.signaling_state()
        );

        let websocket = WebSocket::new(WS_IP_PORT)?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // websocket on open
        // once websocket is open, send a request to open a session
        {
            let session_id_clone = session_id.clone();
            let websocket_clone = websocket.clone();
            let onopen_callback = Closure::wrap(Box::new(move |_| {
                let signal_message = SignalMessage::SessionStartOrJoin(session_id_clone.clone());
                let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
                websocket_clone
                    .send_with_str(&signal_message)
                    .unwrap();
            }) as Box<dyn FnMut(JsValue)>);
            websocket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
        }

        // websocket on message
        // handle message sent by signaling server
        // basically a state automata handling each step in session and webrtc setup
        {
            let session_id_clone = session_id.clone();
            let websocket_clone = websocket.clone();
            let peer_connection_clone = peer_connection.clone();
            let onmessage_callback = Closure::wrap(Box::new(move |ev: MessageEvent| {
                if let Ok(message) = ev.data().dyn_into::<JsString>() {
                    let message = serde_json_wasm::from_str(&String::from(message)).unwrap();

                    let session_id_clone = session_id_clone.clone();
                    let websocket_clone = websocket_clone.clone();
                    let peer_connection_clone = peer_connection_clone.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        handle_websocket_message(message, peer_connection_clone, websocket_clone, session_id_clone).await
                    });
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            websocket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        }

        // peer_connection on ice candidate
        // receive ice candidate from STUN server and send it to client via websocket
        {
            let websocket_clone = websocket.clone();
            let session_id_clone = session_id.clone();
            let onicecandidate_closure =
                Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| {
                    if let Some(candidate) = ev.candidate() {
                        info!(
                            "peer_connection_1.onicecandidate: {:#?}",
                            candidate.candidate()
                        );

                        let ice_candidate_message = {
                            let candidate = candidate.to_json();
                            let candidate = JSON::stringify(&candidate).unwrap();
                            let candidate = String::from(candidate);

                            let signal_message =
                                SignalMessage::IceCandidate(candidate, session_id_clone.clone());
                            serde_json_wasm::to_string(&signal_message).unwrap()
                        };

                        websocket_clone
                            .send_with_str(&ice_candidate_message)
                            .unwrap();
                    }
                })
                    as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
            peer_connection
                .set_onicecandidate(Some(onicecandidate_closure.as_ref().unchecked_ref()));
            onicecandidate_closure.forget();
        }

        // peer_connection on ice connection state change
        {
            let peer_connection_clone = peer_connection.clone();
            let oniceconnectionstatechange_callback = Closure::wrap(Box::new(move || {
                match peer_connection_clone.ice_connection_state() {
                    RtcIceConnectionState::Connected => {
                        // TODO: notify that client can start sending data now
                    }
                    _ => {}
                }
            })
                as Box<dyn FnMut()>);
            peer_connection.set_oniceconnectionstatechange(Some(
                oniceconnectionstatechange_callback.as_ref().unchecked_ref(),
            ));
            oniceconnectionstatechange_callback.forget();
        }

        let data_channel = peer_connection.create_data_channel("data_channel_1");
        console::log_1(&format!("data_channel_1 created: label {:?}", data_channel.label()).into());

        // data_channel.onmessage
        {
            let data_channel_1_clone = data_channel.clone();

            let onmessage_closure =
                Closure::wrap(
                    Box::new(move |ev: MessageEvent| match ev.data().as_string() {
                        Some(message) => {
                            console::log_1(
                                &format!("message to peer connection 1: {:?}", message).into(),
                            );

                            // TODO: handle received message
                            data_channel_1_clone
                                .send_with_str(&format!("Echoing back the message: {:?}", message))
                                .unwrap_or_else(|error| {
                                    console::warn_1(
                                        &format!("Couldn't send to data channel: {:?}", error)
                                            .into(),
                                    );
                                });
                        }
                        None => {}
                    }) as Box<dyn FnMut(MessageEvent)>,
                );
            data_channel.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
            onmessage_closure.forget();
        }

        Ok(Rc::new(RefCell::new(MiniServer {
            peer_connection,
            data_channel,
            session_id,
        })))
    }

    pub fn send_message(&self, message: &str) -> Result<(), JsValue> {
        console::log_1(&format!("server will try to send a message: {:?}", &message).into());
        self.data_channel.send_with_str(message)
    }

}

async fn handle_websocket_message(message: SignalMessage, peer_connection_clone: RtcPeerConnection, websocket_clone: WebSocket, session_id_clone: SessionId) {
    match message {
        SignalMessage::SessionReady(session_id) => {
            let offer = create_sdp_offer(peer_connection_clone.clone()).await.unwrap();
            let signal_message = SignalMessage::SessionStartOrJoin(session_id_clone.clone());
            let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
            websocket_clone
                .send_with_str(&signal_message)
                .unwrap();
        }
        SignalMessage::SdpAnswer(answer, session_id) => {
            // TODO: handle answer provided by the client
        }
        SignalMessage::IceCandidate(ice_candidate, session_id) => {
            // TODO: handle candidate provided by the client
        }
        SignalMessage::Error(error, session_id) => {
            error!("signaling server returned error: session id: {}, error:{}", session_id, error);
        }
        signal_message => {
            error!("mini-server should not have received: {:?}", signal_message);
        }
    }
}
