use std::rc::Rc;
use std::sync::RwLock;

use js_sys::JsString;
use log::{debug, error, info};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{MessageEvent, RtcPeerConnection, RtcPeerConnectionIceEvent};
use web_sys::{
    RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate, RtcIceCandidateInit, RtcSdpType,
    RtcSessionDescriptionInit, WebSocket,
};

use rusty_games_protocol::{SessionId, SignalMessage};

use crate::common::{
    create_sdp_answer, create_sdp_offer, create_stun_peer_connection, IceCandidate, WS_IP_PORT,
};

pub enum ConnectionType {
    Local,
    Stun,
    StunAndTurn,
}

#[derive(Debug, Clone)]
struct NetworkManagerInner {
    session_id: String,
    peer_connection: RtcPeerConnection,
    data_channel: Option<RtcDataChannel>,
}

#[derive(Debug, Clone)]
pub struct NetworkManager {
    inner: Rc<RwLock<NetworkManagerInner>>,
}

impl NetworkManager {
    pub fn new(session_id: SessionId, connection_type: ConnectionType) -> Result<Self, JsValue> {
        let peer_connection = match connection_type {
            ConnectionType::Local => RtcPeerConnection::new()?,
            ConnectionType::Stun => create_stun_peer_connection()?,
            ConnectionType::StunAndTurn => unimplemented!("no turn server yet!"),
        };
        debug!(
            "peer connections created, signaling state: {:?}",
            peer_connection.signaling_state()
        );

        Ok(NetworkManager {
            inner: Rc::new(RwLock::new(NetworkManagerInner {
                session_id,
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
            // peer_connection on data channel
            {
                let self_clone = self.clone();
                let on_open_callback = on_open_callback;
                let on_message_callback = on_message_callback;
                let on_datachannel =
                    Closure::wrap(Box::new(move |data_channel_event: RtcDataChannelEvent| {
                        info!("received data channel");
                        let data_channel = data_channel_event.channel();

                        set_data_channel_on_open(&data_channel, on_open_callback.clone());
                        set_data_channel_on_error(&data_channel);
                        set_data_channel_on_message(&data_channel, on_message_callback.clone());

                        self_clone.inner.write().unwrap().data_channel = Some(data_channel);
                    })
                        as Box<dyn FnMut(RtcDataChannelEvent)>);
                peer_connection.set_ondatachannel(Some(on_datachannel.as_ref().unchecked_ref()));
                on_datachannel.forget();
            }
        }

        let websocket = WebSocket::new(WS_IP_PORT)?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // peer_connection on ice candidate
        set_peer_connection_on_ice_candidate(
            &peer_connection,
            websocket.clone(),
            session_id.clone(),
        );

        // peer_connection on ice connection state change
        set_peer_connection_on_ice_connection_state_change(&peer_connection);

        // peer_connection on ice gathering state change
        {
            let peer_connection_clone = peer_connection.clone();
            let on_ice_gathering_state_change = Closure::wrap(Box::new(move || {
                debug!(
                    "ice gathering state: {:?}",
                    peer_connection_clone.ice_gathering_state()
                );
            }) as Box<dyn FnMut()>);
            peer_connection.set_onicegatheringstatechange(Some(
                on_ice_gathering_state_change.as_ref().unchecked_ref(),
            ));
            on_ice_gathering_state_change.forget();
        }

        // peer connection on negotiation needed
        {
            let on_negotiation_needed = Closure::wrap(Box::new(move || {
                debug!("on negotiation needed event occurred");
            }) as Box<dyn FnMut()>);
            peer_connection
                .set_onnegotiationneeded(Some(on_negotiation_needed.as_ref().unchecked_ref()));
            on_negotiation_needed.forget();
        }

        // websocket on open
        // once websocket is open, send a request to open a session
        {
            let session_id_clone = session_id;
            let websocket_clone = websocket.clone();
            let onopen_callback = Closure::wrap(Box::new(move |_| {
                let signal_message = SignalMessage::SessionStartOrJoin(session_id_clone.clone());
                let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
                websocket_clone.send_with_str(&signal_message).unwrap();
            }) as Box<dyn FnMut(JsValue)>);
            websocket.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
        }

        // websocket on message
        // handle message sent by signaling server
        // basically a state automata handling each step in session and webrtc setup
        {
            let websocket_clone = websocket.clone();
            let peer_connection_clone = peer_connection;
            let onmessage_callback = Closure::wrap(Box::new(move |ev: MessageEvent| {
                if let Ok(message) = ev.data().dyn_into::<JsString>() {
                    let message = serde_json_wasm::from_str(&String::from(message)).unwrap();

                    let websocket_clone = websocket_clone.clone();
                    let peer_connection_clone = peer_connection_clone.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        handle_websocket_message(
                            message,
                            peer_connection_clone,
                            websocket_clone,
                            is_host,
                        )
                        .await
                        .unwrap_or_else(|error| {
                            error!("error handling websocket message: {:?}", error);
                        })
                    });
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            websocket.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        }

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

fn set_data_channel_on_message(
    data_channel: &RtcDataChannel,
    mut on_message_callback: impl FnMut(String) + 'static,
) {
    let datachannel_on_message = Closure::wrap(Box::new(move |ev: MessageEvent| {
        if let Some(message) = ev.data().as_string() {
            debug!(
                "message from datachannel (will call on_message): {:?}",
                message
            );
            on_message_callback(message);
        }
    }) as Box<dyn FnMut(MessageEvent)>);
    data_channel.set_onmessage(Some(datachannel_on_message.as_ref().unchecked_ref()));
    datachannel_on_message.forget();
}

fn set_data_channel_on_error(data_channel: &RtcDataChannel) {
    let onerror = Closure::wrap(Box::new(move |data_channel_error| {
        error!("data channel error: {:?}", data_channel_error);
    }) as Box<dyn FnMut(JsValue)>);
    data_channel.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();
}

fn set_data_channel_on_open(
    data_channel: &RtcDataChannel,
    mut on_open_callback: impl FnMut() + 'static,
) {
    let onopen_callback = Closure::wrap(Box::new(move |_| {
        debug!("data channel is now open, calling on_open!");
        on_open_callback();
    }) as Box<dyn FnMut(JsValue)>);
    data_channel.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
    onopen_callback.forget();
}

fn set_peer_connection_on_ice_connection_state_change(peer_connection: &RtcPeerConnection) {
    let peer_connection_clone = peer_connection.clone();
    let on_ice_connection_state_change = Closure::wrap(Box::new(move || {
        debug!(
            "connection state change: {:?}",
            peer_connection_clone.ice_connection_state()
        )
    }) as Box<dyn FnMut()>);
    peer_connection.set_oniceconnectionstatechange(Some(
        on_ice_connection_state_change.as_ref().unchecked_ref(),
    ));
    on_ice_connection_state_change.forget();
}

fn set_peer_connection_on_ice_candidate(
    peer_connection: &RtcPeerConnection,
    websocket_clone: WebSocket,
    session_id_clone: SessionId,
) {
    let on_ice_candidate = Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| {
        if let Some(candidate) = ev.candidate() {
            let signaled_candidate = IceCandidate {
                candidate: candidate.candidate(),
                sdp_mid: candidate.sdp_mid().unwrap(),
                sdp_m_line_index: candidate.sdp_m_line_index().unwrap(),
            };
            debug!("signaled candidate: {:#?}", signaled_candidate);
            let signaled_candidate = serde_json_wasm::to_string(&signaled_candidate).unwrap();

            let signal_message =
                SignalMessage::IceCandidate(signaled_candidate, session_id_clone.clone());
            let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();

            websocket_clone.send_with_str(&signal_message).unwrap();
        }
    }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
    peer_connection.set_onicecandidate(Some(on_ice_candidate.as_ref().unchecked_ref()));
    on_ice_candidate.forget();
}

async fn handle_websocket_message(
    message: SignalMessage,
    peer_connection: RtcPeerConnection,
    websocket: WebSocket,
    is_host: bool,
) -> Result<(), JsValue> {
    match message {
        SignalMessage::SessionStartOrJoin(_session_id) => {
            error!("error, SessionStartOrJoin should only be sent by peers to signaling server");
        }
        SignalMessage::SessionReady(session_id, is_host) => {
            info!("peer received info that session is ready {}", session_id);
            if is_host {
                let offer = create_sdp_offer(peer_connection.clone()).await?;
                let signal_message = SignalMessage::SdpOffer(offer, session_id.clone());
                let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
                websocket.send_with_str(&signal_message)?;
                debug!("(is_host: {}) sent an offer successfully", is_host);
            }
        }
        SignalMessage::SdpOffer(offer, session_id) => {
            let answer = create_sdp_answer(peer_connection.clone(), offer)
                .await
                .unwrap();
            debug!(
                "(is_host: {}) received an offer and created an answer: {}",
                is_host, answer
            );
            let signal_message = SignalMessage::SdpAnswer(answer, session_id);
            let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
            websocket.send_with_str(&signal_message).unwrap();
        }
        SignalMessage::SdpAnswer(answer, session_id) => {
            let mut remote_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            remote_session_description.sdp(&answer);
            JsFuture::from(peer_connection.set_remote_description(&remote_session_description))
                .await
                .unwrap();
            debug!(
                "received answer from peer and set remote description: {}, {}",
                answer, session_id
            );
        }
        SignalMessage::IceCandidate(ice_candidate, _session_id) => {
            debug!(
                "(is host: {}) peer received ice candidate: {}",
                is_host, &ice_candidate
            );
            let ice_candidate = serde_json_wasm::from_str::<IceCandidate>(&ice_candidate).unwrap();

            let mut rtc_candidate = RtcIceCandidateInit::new("");
            rtc_candidate.candidate(&ice_candidate.candidate);
            rtc_candidate.sdp_m_line_index(Some(ice_candidate.sdp_m_line_index));
            rtc_candidate.sdp_mid(Some(&ice_candidate.sdp_mid));

            let rtc_candidate = RtcIceCandidate::new(&rtc_candidate).unwrap();
            JsFuture::from(
                peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&rtc_candidate)),
            )
            .await
            .unwrap();
            debug!(
                "(is host: {}) added ice candidate {:?}",
                is_host, ice_candidate
            );
        }
        SignalMessage::Error(error, session_id) => {
            error!(
                "signaling server returned error: session id: {}, error:{}",
                session_id, error
            );
        }
    }

    Ok(())
}
