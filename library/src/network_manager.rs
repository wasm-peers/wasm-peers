use js_sys::{JsString, JSON};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue, UnwrapThrowExt};

use log::{debug, error, info, warn};
use rusty_games_protocol::{SessionId, SignalMessage};
use wasm_bindgen_futures::JsFuture;
use web_sys::{MessageEvent, RtcIceGatheringState, RtcPeerConnection, RtcPeerConnectionIceEvent};
use web_sys::{
    RtcDataChannel, RtcDataChannelEvent, RtcIceCandidate, RtcIceCandidateInit,
    RtcIceConnectionState, RtcSdpType, RtcSessionDescriptionInit, WebSocket,
};

use crate::common::{
    create_sdp_answer, create_sdp_offer, create_stun_peer_connection, IceCandidate, WS_IP_PORT,
};

pub enum ConnectionType {
    Local,
    Stun,
    StunAndTurn,
}

#[derive(Debug)]
pub struct NetworkManager {
    session_id: String,
    peer_connection: RtcPeerConnection,
    data_channel: Option<RtcDataChannel>,
}

impl NetworkManager {
    pub fn start(
        session_id: String,
        connection_type: ConnectionType,
        is_host: bool,
    ) -> Result<Rc<RefCell<Self>>, JsValue> {
        let peer_connection = match connection_type {
            ConnectionType::Local => RtcPeerConnection::new()?,
            ConnectionType::Stun => create_stun_peer_connection()?,
            ConnectionType::StunAndTurn => unimplemented!("no turn server yet!"),
        };
        info!(
            "peer connections created, states: {:?}",
            peer_connection.signaling_state()
        );

        let network_manager = Rc::new(RefCell::new(NetworkManager {
            session_id: session_id.clone(),
            peer_connection: peer_connection.clone(),
            data_channel: None,
        }));

        let websocket = WebSocket::new(WS_IP_PORT)?;
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        if is_host {
            // peer connection on negotiation needed
            let peer_connection_clone = peer_connection.clone();
            let session_id_clone = session_id.clone();
            let websocket_clone = websocket.clone();
            {
                let on_negotiation_needed = Closure::wrap(Box::new(move || {
                    warn!("on negotiation needed event occurred!");
                    let peer_connection_clone = peer_connection_clone.clone();
                    let session_id_clone = session_id_clone.clone();
                    let websocket_clone = websocket_clone.clone();
                    // TODO: only do this if websocket is open!!!
                    info!("(on negotiation needed) websocket ready state: {}", websocket_clone.ready_state());
                    if websocket_clone.ready_state() == 1 {
                        info!("(on negotiation needed) websocket is ready");
                        wasm_bindgen_futures::spawn_local(async move {
                            let offer = create_sdp_offer(peer_connection_clone).await.unwrap();
                            info!("(on negotiation needed, is_host: {}) created an offer: {}", is_host, offer);
                            let signal_message = SignalMessage::SdpOffer(offer, session_id_clone.clone());
                            let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
                            match websocket_clone.send_with_str(&signal_message) {
                                Ok(_) => info!("(on negotiation needed) websocket send offer successful"),
                                Err(error) => error!("(on negotiation needed) websocket not yet ready"),
                            }
                            info!("(on negotiation needed) sent the offer to peer successfully: {}", session_id_clone);
                        });
                    }
                }) as Box<dyn FnMut()>);
                peer_connection
                    .set_onnegotiationneeded(Some(on_negotiation_needed.as_ref().unchecked_ref()));
                on_negotiation_needed.forget();
            }

            let data_channel =
                peer_connection.create_data_channel(&format!("data_channel_{}", &session_id));
            info!("data_channel created: label {:?}", data_channel.label());

            // data_channel.onopen
            {
                let data_channel_clone = data_channel.clone();
                let onopen_callback = Closure::wrap(Box::new(move |_| {
                    info!("data channel is now open!");
                    // TODO: inform server that data channel is open and ready for transmission
                }) as Box<dyn FnMut(JsValue)>);
                data_channel.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
                onopen_callback.forget();
            }

            // data_channel.onerror
            {
                let onerror = Closure::wrap(Box::new(move |data_channel_error| {
                    error!("data channel error: {:?}", data_channel_error);
                }) as Box<dyn FnMut(JsValue)>);
                data_channel.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                onerror.forget();
            }

            // data_channel.onmessage
            {
                let data_channel_clone = data_channel.clone();
                let onmessage_closure =
                    Closure::wrap(
                        Box::new(move |ev: MessageEvent| match ev.data().as_string() {
                            Some(message) => {
                                info!("message to peer connection 1: {:?}", message);

                                // TODO: handle received message
                                data_channel_clone
                                    .send_with_str(&format!(
                                        "Echoing back the message: {:?}",
                                        message
                                    ))
                                    .unwrap_or_else(|error| {
                                        warn!("Couldn't send to data channel: {:?}", error);
                                    });
                            }
                            None => {}
                        }) as Box<dyn FnMut(MessageEvent)>,
                    );
                data_channel.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
                onmessage_closure.forget();
            }

            network_manager.borrow_mut().data_channel = Some(data_channel);
            debug!("network manager: {:#?}", &network_manager);
        } else {
            info!("setting on data channel callback");
            // peer_connection on data channel
            {
                let network_manager_clone = network_manager.clone();
                let ondatachannel_callback =
                    Closure::wrap(Box::new(move |data_channel_event: RtcDataChannelEvent| {
                        info!("received data channel");
                        let data_channel = data_channel_event.channel();
                        network_manager_clone.borrow_mut().data_channel =
                            Some(data_channel.clone());

                        // data_channel.onopen
                        {
                            let data_channel_clone = data_channel.clone();
                            let onopen_callback = Closure::wrap(Box::new(move |_| {
                                info!("data channel is now open!");
                                // TODO: inform server that data channel is open and ready for transmission
                            })
                                as Box<dyn FnMut(JsValue)>);
                            data_channel.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
                            onopen_callback.forget();
                        }

                        // data_channel.onerror
                        {
                            let onerror = Closure::wrap(Box::new(move |data_channel_error| {
                                error!("data channel error: {:?}", data_channel_error);
                            })
                                as Box<dyn FnMut(JsValue)>);
                            data_channel.set_onerror(Some(onerror.as_ref().unchecked_ref()));
                            onerror.forget();
                        }

                        // data_channel.onmessage
                        {
                            let data_channel_clone = data_channel.clone();
                            let onmessage_closure =
                                Closure::wrap(Box::new(move |ev: MessageEvent| {
                                    match ev.data().as_string() {
                                        Some(message) => {
                                            info!("message to peer connection 1: {:?}", message);

                                            // TODO: handle received message
                                            data_channel_clone
                                                .send_with_str(&format!(
                                                    "Echoing back the message: {:?}",
                                                    message
                                                ))
                                                .unwrap_or_else(|error| {
                                                    warn!(
                                                        "Couldn't send to data channel: {:?}",
                                                        error
                                                    );
                                                });
                                        }
                                        None => {}
                                    }
                                })
                                    as Box<dyn FnMut(MessageEvent)>);
                            data_channel
                                .set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
                            onmessage_closure.forget();
                        }
                    })
                        as Box<dyn FnMut(RtcDataChannelEvent)>);
                peer_connection
                    .set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
                ondatachannel_callback.forget();
            }
        }

        // websocket on open
        // once websocket is open, send a request to open a session
        {
            let session_id_clone = session_id.clone();
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
                        handle_websocket_message(
                            message,
                            peer_connection_clone,
                            websocket_clone,
                            session_id_clone,
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

        // peer_connection on ice candidate
        // receive ice candidate from STUN server and send it to client via websocket
        {
            let websocket_clone = websocket.clone();
            let session_id_clone = session_id.clone();
            let onicecandidate_closure =
                Closure::wrap(Box::new(move |ev: RtcPeerConnectionIceEvent| {
                    if let Some(candidate) = ev.candidate() {
                        let signaled_candidate = IceCandidate {
                            candidate: candidate.candidate(),
                            sdp_mid: candidate.sdp_mid().unwrap(),
                            sdp_m_line_index: candidate.sdp_m_line_index().unwrap(),
                        };
                        info!("signaled candidate: {:#?}", signaled_candidate);
                        let signaled_candidate =
                            serde_json_wasm::to_string(&signaled_candidate).unwrap();

                        let signal_message = SignalMessage::IceCandidate(
                            signaled_candidate,
                            session_id_clone.clone(),
                        );
                        let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();

                        websocket_clone.send_with_str(&signal_message).unwrap();
                    } else {
                        warn!("no ICE candidate found!");
                    }
                })
                    as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
            peer_connection
                .set_onicecandidate(Some(onicecandidate_closure.as_ref().unchecked_ref()));
            onicecandidate_closure.forget();
        }

        // peer_connection on ice connection state change
        {
            let network_manager_clone = network_manager.clone();
            let peer_connection_clone = peer_connection.clone();
            let oniceconnectionstatechange_callback =
                Closure::wrap(
                    Box::new(move || match peer_connection_clone.ice_connection_state() {
                        RtcIceConnectionState::Connected => {
                            info!("peer connection changed state to CONNECTED");
                            debug!("network manager: {:#?}", &network_manager_clone);
                        }
                        state_change => {
                            warn!("unhandled connection state change: {:?}", state_change);
                        }
                    }) as Box<dyn FnMut()>,
                );
            peer_connection.set_oniceconnectionstatechange(Some(
                oniceconnectionstatechange_callback.as_ref().unchecked_ref(),
            ));
            oniceconnectionstatechange_callback.forget();
        }

        // peer_connection on ice gathering state change
        {
            let peer_connection_clone = peer_connection.clone();
            let onicegatheringstatechange_callback =
                Closure::wrap(
                    Box::new(move || match peer_connection_clone.ice_gathering_state() {
                        state => {
                            info!("ice gathering state: {:?}", state);
                        }
                    }) as Box<dyn FnMut()>,
                );
            peer_connection.set_onicegatheringstatechange(Some(
                onicegatheringstatechange_callback.as_ref().unchecked_ref(),
            ));
            onicegatheringstatechange_callback.forget();
        }

        Ok(network_manager)
    }

    pub fn send_message(&self, message: &str) -> Result<(), JsValue> {
        info!("server will try to send a message: {:?}", &message);
        self.data_channel.as_ref().unwrap().send_with_str(message)
    }
}

async fn handle_websocket_message(
    message: SignalMessage,
    peer_connection: RtcPeerConnection,
    websocket: WebSocket,
    session_id: SessionId,
    is_host: bool,
) -> Result<(), JsValue> {
    match message {
        SignalMessage::SessionStartOrJoin(session_id) => {
            error!("error, SessionStartOrJoin should only be sent by peers to signaling server");
        }
        SignalMessage::SessionReady(session_id, is_host) => {
            info!("peer received info that session is ready {}", session_id);
            if is_host {
                let offer = create_sdp_offer(peer_connection.clone()).await?;
                info!("(is_host: {}) created an offer: {}", is_host, offer);
                let signal_message = SignalMessage::SdpOffer(offer, session_id.clone());
                let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
                websocket.send_with_str(&signal_message)?;
                info!("sent the offer to peer successfully: {}", session_id);
            }
        }
        SignalMessage::SdpOffer(offer, session_id) => {
            info!("received offer from peer: {}, {}", offer, session_id);
            let answer = create_sdp_answer(peer_connection.clone(), offer)
                .await
                .unwrap();
            info!("created an answer: {}", answer);
            let signal_message = SignalMessage::SdpAnswer(answer, session_id);
            let signal_message = serde_json_wasm::to_string(&signal_message).unwrap();
            websocket.send_with_str(&signal_message).unwrap();
        }
        SignalMessage::SdpAnswer(answer, session_id) => {
            info!("received answer from peer: {}, {}", answer, session_id);
            let mut remote_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
            remote_session_description.sdp(&answer);
            JsFuture::from(peer_connection.set_remote_description(&remote_session_description))
                .await
                .unwrap();
            info!("set remote description");
        }
        SignalMessage::IceCandidate(ice_candidate, session_id) => {
            info!(
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
            info!(
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
