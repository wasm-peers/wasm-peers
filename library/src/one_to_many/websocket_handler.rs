use anyhow::anyhow;
use log::{debug, error, info};
use serde::de::DeserializeOwned;
use wasm_bindgen_futures::JsFuture;
use wasm_peers_protocol::one_to_many::SignalMessage;
use wasm_peers_protocol::{SessionId, UserId};
use web_sys::{
    RtcDataChannelInit, RtcIceCandidate, RtcIceCandidateInit, RtcSdpType,
    RtcSessionDescriptionInit, WebSocket,
};

use crate::one_to_many::callbacks::{
    set_data_channel_on_error, set_data_channel_on_message, set_data_channel_on_open,
    set_peer_connection_on_data_channel, set_peer_connection_on_ice_candidate,
    set_peer_connection_on_ice_connection_state_change,
};
use crate::one_to_many::{Connection, NetworkManager};
use crate::utils::{
    create_peer_connection, create_sdp_answer, create_sdp_offer,
    set_peer_connection_on_ice_gathering_state_change, set_peer_connection_on_negotiation_needed,
};

/// Basically a finite state machine spread across host, client and signaling server
/// handling each step in session and then `WebRTC` setup.
pub async fn handle_websocket_message<T: DeserializeOwned>(
    network_manager: NetworkManager,
    message: SignalMessage,
    websocket: WebSocket,
    max_retransmits: u16,
    on_open_callback: impl FnMut(UserId) + Clone + 'static,
    on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
    is_host: bool,
) -> crate::Result<()> {
    match message {
        SignalMessage::SessionJoin(_session_id, _peer_id) => {
            error!("error, SessionStartOrJoin should only be sent by peers to signaling server");
        }
        SignalMessage::SessionReady(session_id, peer_id) => {
            session_ready(
                network_manager,
                websocket,
                max_retransmits,
                on_open_callback,
                on_message_callback,
                is_host,
                session_id,
                peer_id,
            )
            .await?;
        }
        SignalMessage::SdpOffer(session_id, peer_id, offer) => {
            sdp_offer(
                network_manager,
                websocket,
                on_open_callback,
                on_message_callback,
                is_host,
                session_id,
                peer_id,
                offer,
            )
            .await?;
        }
        SignalMessage::SdpAnswer(session_id, user_id, answer) => {
            let peer_connection = network_manager
                .inner
                .borrow()
                .connections
                .get(&user_id)
                .map(Clone::clone)
                .map(|connection| connection.peer_connection)
                .ok_or_else(|| {
                    anyhow!(
                        "(is_host: {}) no connection to send answer for given user_id: {:?}",
                        is_host,
                        &user_id
                    )
                })?;
            let mut remote_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            remote_session_description.sdp(&answer);
            JsFuture::from(peer_connection.set_remote_description(&remote_session_description))
                .await
                .expect("failed to set remote description");
            debug!(
                "received answer from peer and set remote description: {}, {:?}",
                answer, session_id
            );
        }
        SignalMessage::IceCandidate(_session_id, user_id, ice_candidate) => {
            let peer_connection = network_manager
                .inner
                .borrow()
                .connections
                .get(&user_id)
                .map(Clone::clone)
                .map(|connection| connection.peer_connection)
                .ok_or_else(|| {
                    anyhow!(
                        "no connection to send ice candidate to for given user_id: {:?}",
                        &user_id
                    )
                })?;
            debug!("peer received ice candidate: {:?}", &ice_candidate);

            let mut rtc_candidate = RtcIceCandidateInit::new("");
            rtc_candidate.candidate(&ice_candidate.candidate);
            rtc_candidate.sdp_m_line_index(ice_candidate.sdp_m_line_index);
            rtc_candidate.sdp_mid(ice_candidate.sdp_mid.as_deref());

            let rtc_candidate = RtcIceCandidate::new(&rtc_candidate)
                .map_err(|err| anyhow!("failed to create RTC ICE candidate: {:?}", err))?;
            JsFuture::from(
                peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&rtc_candidate)),
            )
            .await
            .map_err(|err| {
                anyhow!(
                    "failed to add ice candidate with optional RTC ICE candidate: {:?}",
                    err
                )
            })?;
            debug!("added ice candidate {:?}", ice_candidate);
        }
        SignalMessage::Error(session_id, user_id, error) => {
            error!(
                "signaling server returned error: session id: {session_id:?}, user_id: \
                 {user_id:?}, error: {error}",
            );
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn session_ready<T: DeserializeOwned>(
    network_manager: NetworkManager,
    websocket: WebSocket,
    max_retransmits: u16,
    on_open_callback: impl FnMut(UserId) + Clone + 'static,
    on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
    is_host: bool,
    session_id: SessionId,
    peer_id: UserId,
) -> crate::Result<()> {
    info!(
        "peer received info that session with {:?} is ready {:?}",
        peer_id, session_id
    );
    let peer_connection = create_peer_connection(&network_manager.inner.borrow().connection_type)?;
    set_peer_connection_on_data_channel(
        &peer_connection,
        peer_id,
        network_manager.clone(),
        on_open_callback.clone(),
        on_message_callback.clone(),
    );
    set_peer_connection_on_ice_candidate(&peer_connection, peer_id, websocket.clone(), session_id);
    set_peer_connection_on_ice_connection_state_change(&peer_connection);
    set_peer_connection_on_ice_gathering_state_change(&peer_connection);
    set_peer_connection_on_negotiation_needed(&peer_connection);

    let mut init = RtcDataChannelInit::new();
    init.max_retransmits(max_retransmits);
    init.ordered(false);
    let data_channel = peer_connection
        .create_data_channel_with_data_channel_dict(&format!("{}-{}", session_id, peer_id), &init);

    set_data_channel_on_open(&data_channel, peer_id, on_open_callback.clone());
    set_data_channel_on_error(&data_channel);
    set_data_channel_on_message(&data_channel, peer_id, on_message_callback.clone());

    let offer = create_sdp_offer(&peer_connection).await?;
    let signal_message = SignalMessage::SdpOffer(session_id, peer_id, offer);
    let signal_message = rmp_serde::to_vec(&signal_message)?;
    websocket
        .send_with_u8_array(&signal_message)
        .map_err(|err| anyhow!("failed to send message across the websocket: {:?}", err))?;
    network_manager.inner.borrow_mut().connections.insert(
        peer_id,
        Connection::new(peer_connection.clone(), Some(data_channel.clone())),
    );
    debug!(
        "(is_host: {}) sent an offer to {:?} successfully",
        is_host, peer_id
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn sdp_offer<T: DeserializeOwned>(
    network_manager: NetworkManager,
    websocket: WebSocket,
    on_open_callback: impl FnMut(UserId) + Clone + 'static,
    on_message_callback: impl FnMut(UserId, T) + Clone + 'static,
    is_host: bool,
    session_id: SessionId,
    peer_id: UserId,
    offer: String,
) -> crate::Result<()> {
    // non-host peer received an offer
    let peer_connection = create_peer_connection(&network_manager.inner.borrow().connection_type)?;
    set_peer_connection_on_data_channel(
        &peer_connection,
        peer_id,
        network_manager.clone(),
        on_open_callback.clone(),
        on_message_callback.clone(),
    );
    set_peer_connection_on_ice_candidate(&peer_connection, peer_id, websocket.clone(), session_id);
    set_peer_connection_on_ice_connection_state_change(&peer_connection);
    set_peer_connection_on_ice_gathering_state_change(&peer_connection);
    set_peer_connection_on_negotiation_needed(&peer_connection);

    network_manager
        .inner
        .borrow_mut()
        .connections
        .insert(peer_id, Connection::new(peer_connection.clone(), None));
    debug!(
        "(is_host: {}) added connection for {:?} successfully",
        is_host, peer_id
    );

    let answer = create_sdp_answer(&peer_connection, offer)
        .await
        .expect("failed to create SDP answer");
    debug!(
        "received an offer from {:?} and created an answer: {}",
        peer_id, answer
    );
    let signal_message = SignalMessage::SdpAnswer(session_id, peer_id, answer);
    let signal_message =
        rmp_serde::to_vec(&signal_message).expect("failed to serialize SignalMessage");
    websocket
        .send_with_u8_array(&signal_message)
        .expect("failed to send SPD answer to signaling server");
    Ok(())
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use mockall::mock;
//     use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
//     use web_sys::{RtcIceConnectionState, RtcIceGatheringState};
//     use wasm_peers_protocol::SessionId;
//
//     wasm_bindgen_test_configure!(run_in_browser);
//
//     mock! {
//         WebSocket {}
//     }
//
//     // #[wasm_bindgen_test]
//     async fn test_handle_session_ready_signal_is_successful() {
//         let message = SignalMessage::SessionReady(SessionId::new("dummy-session-id".to_string()), true);
//         let peer_connection = RtcPeerConnection::new().unwrap();
//
//         // TODO(tkarwowski): this should be mocked, but how do you pass a mock to a function expecting different type?
//         //  I could introduce a trait, implement it for web_sys::WebSocket and MockWebSocket as well,
//         //  but that's a lot of work...
//         //  This is a integration test for now.
//         let websocket = WebSocket::new("ws://0.0.0.0:9001/ws")
//             .expect("local signaling server instance was not found");
//         websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);
//
//         // FIXME(tkarwowski): this fails because peer_connection state gets modified in other tests
//         handle_websocket_message(message, peer_connection.clone(), websocket)
//             .await
//             .unwrap();
//         assert!(peer_connection.local_description().is_some());
//     }
// }
