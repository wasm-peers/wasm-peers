use crate::network_manager::utils::{create_sdp_answer, create_sdp_offer, IceCandidate};
use ::log::{debug, error, info};
use rusty_games_protocol::SignalMessage;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit,
    WebSocket,
};

/// basically a state automata spread across host, client and signaling server
/// handling each step in session and then WebRTC setup
pub(crate) async fn handle_websocket_message(
    message: SignalMessage,
    peer_connection: RtcPeerConnection,
    websocket: WebSocket,
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
                let signal_message = serde_json_wasm::to_string(&signal_message)
                    .expect("failed to serialize SignalMessage");
                websocket.send_with_str(&signal_message)?;
                debug!("(is_host: {}) sent an offer successfully", is_host);
            }
        }
        SignalMessage::SdpOffer(offer, session_id) => {
            let answer = create_sdp_answer(peer_connection.clone(), offer)
                .await
                .expect("failed to create SDP answer");
            debug!(
                "received an offer and created an answer: {}",
                answer
            );
            let signal_message = SignalMessage::SdpAnswer(answer, session_id);
            let signal_message = serde_json_wasm::to_string(&signal_message)
                .expect("failed to serialize SignalMessage");
            websocket
                .send_with_str(&signal_message)
                .expect("failed to send SPD answer to signaling server");
        }
        SignalMessage::SdpAnswer(answer, session_id) => {
            let mut remote_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
            remote_session_description.sdp(&answer);
            JsFuture::from(peer_connection.set_remote_description(&remote_session_description))
                .await
                .expect("failed to set remote descripiton");
            debug!(
                "received answer from peer and set remote description: {}, {}",
                answer, session_id
            );
        }
        SignalMessage::IceCandidate(ice_candidate, _session_id) => {
            debug!(
                "peer received ice candidate: {}",
                &ice_candidate
            );
            let ice_candidate = serde_json_wasm::from_str::<IceCandidate>(&ice_candidate)
                .expect("failed to deserialize IceCandidate");

            let mut rtc_candidate = RtcIceCandidateInit::new("");
            rtc_candidate.candidate(&ice_candidate.candidate);
            rtc_candidate.sdp_m_line_index(ice_candidate.sdp_m_line_index);
            rtc_candidate.sdp_mid(ice_candidate.sdp_mid.as_deref());

            let rtc_candidate =
                RtcIceCandidate::new(&rtc_candidate).expect("failed to create new RtcIceCandidate");
            JsFuture::from(
                peer_connection.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&rtc_candidate)),
            )
            .await
            .expect("failed to add ICE candidate");
            debug!(
                "added ice candidate {:?}",
                ice_candidate
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

#[cfg(test)]
mod test {
    use super::*;
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{RtcIceConnectionState, RtcIceGatheringState};
    use mockall::mock;

    wasm_bindgen_test_configure!(run_in_browser);

    mock! {
        WebSocket {}
    }

    // #[wasm_bindgen_test]
    async fn test_handle_session_ready_signal_is_successful() {
        let message = SignalMessage::SessionReady("dummy-session-id".to_string(), true);
        let peer_connection = RtcPeerConnection::new().unwrap();

        // TODO: this should be mocked, but how do you pass a mock to a function expecting different type?
        //  I could introduce a trait, implement it for web_sys::WebSocket and MockWebSocket as well,
        //  but that's a lot of work...
        //  This is a integration test for now.
        let websocket = WebSocket::new("ws://0.0.0.0:9001/ws").expect("local signaling server instance was not found");
        websocket.set_binary_type(web_sys::BinaryType::Arraybuffer);

        // FIXME: this fails because peer_connection state gets modified in other tests
        handle_websocket_message(message, peer_connection.clone(), websocket).await.unwrap();
        assert!(peer_connection.local_description().is_some());
    }
}