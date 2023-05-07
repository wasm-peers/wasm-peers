use anyhow::anyhow;
use js_sys::{Array, Object, Reflect};
use log::debug;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use wasm_peers_protocol::SessionId;
use web_sys::{RtcConfiguration, RtcPeerConnection, RtcSdpType, RtcSessionDescriptionInit};

/// Returns a new `SessionId` instance that can be used to identify a session by signaling server.
#[must_use]
pub fn get_random_session_id() -> SessionId {
    SessionId::new(uuid::Uuid::new_v4().as_u128())
}

/// Specifies what kind of peer connection to create
#[derive(Debug, Clone)]
pub enum ConnectionType {
    /// Within local network
    Local,
    /// Setup with STUN server, WAN capabilities but can fail
    Stun { urls: String },
    /// Setup with STUN and TURN servers and fallback to TURN if needed, most stable connection
    StunAndTurn {
        stun_urls: String,
        turn_urls: String,
        username: String,
        credential: String,
    },
}

pub fn create_peer_connection(
    connection_type: &ConnectionType,
) -> crate::Result<RtcPeerConnection> {
    match *connection_type {
        ConnectionType::Local => RtcPeerConnection::new()
            .map_err(|err| anyhow!("failed to create RTC peer connection: {:?}", err)),
        ConnectionType::Stun { ref urls } => {
            let ice_servers = Array::new();
            {
                let server_entry = Object::new();

                Reflect::set(&server_entry, &"urls".into(), &urls.into()).map_err(|err| {
                    anyhow!(
                        "failed to set 'urls' key on turn server entry object: {:?}",
                        err
                    )
                })?;

                ice_servers.push(&server_entry);
            }

            let mut rtc_configuration = RtcConfiguration::new();
            rtc_configuration.ice_servers(&ice_servers);

            RtcPeerConnection::new_with_configuration(&rtc_configuration)
                .map_err(|err| anyhow!("failed to create RTC peer connection: {:?}", err))
        }
        ConnectionType::StunAndTurn {
            ref stun_urls,
            ref turn_urls,
            ref username,
            ref credential,
        } => {
            let ice_servers = Array::new();
            {
                let stun_server_entry = Object::new();

                Reflect::set(&stun_server_entry, &"urls".into(), &stun_urls.into()).map_err(
                    |err| {
                        anyhow!(
                            "failed to set 'urls' key on turn server entry object: {:?}",
                            err
                        )
                    },
                )?;

                ice_servers.push(&stun_server_entry);
            }
            {
                let turn_server_entry = Object::new();

                Reflect::set(&turn_server_entry, &"urls".into(), &turn_urls.into()).map_err(
                    |err| {
                        anyhow!(
                            "failed to set 'urls' key on turn server entry object: {:?}",
                            err
                        )
                    },
                )?;
                Reflect::set(&turn_server_entry, &"username".into(), &username.into()).map_err(
                    |err| {
                        anyhow!(
                            "failed to set 'username' key on turn server entry object: {:?}",
                            err
                        )
                    },
                )?;
                Reflect::set(&turn_server_entry, &"credential".into(), &credential.into())
                    .map_err(|err| {
                        anyhow!(
                            "failed to set 'credential' key on turn server entry object: {:?}",
                            err
                        )
                    })?;

                ice_servers.push(&turn_server_entry);
            }

            let mut rtc_configuration = RtcConfiguration::new();
            rtc_configuration.ice_servers(&ice_servers);

            RtcPeerConnection::new_with_configuration(&rtc_configuration)
                .map_err(|err| anyhow!("failed to create RTC peer connection: {:?}", err))
        }
    }
}

pub async fn create_sdp_offer(peer_connection: &RtcPeerConnection) -> crate::Result<String> {
    let offer = JsFuture::from(peer_connection.create_offer())
        .await
        .map_err(|error| {
            anyhow!(
                "failed to create an SDP offer: {}",
                error.as_string().unwrap_or_default()
            )
        })?;
    let offer = Reflect::get(&offer, &JsValue::from_str("sdp"))
        .map_err(|err| {
            anyhow!(
                "failed to get value for 'sdp' key from offer object: {:?}",
                err
            )
        })?
        .as_string()
        .ok_or_else(|| anyhow!("no 'sdp' key in offer object"))?;
    let mut local_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    local_session_description.sdp(&offer);
    JsFuture::from(peer_connection.set_local_description(&local_session_description))
        .await
        .map_err(|error| {
            anyhow!(
                "failed to set local description: {}",
                error.as_string().unwrap_or_default()
            )
        })?;

    Ok(offer)
}

pub async fn create_sdp_answer(
    peer_connection: &RtcPeerConnection,
    offer: String,
) -> crate::Result<String> {
    let mut remote_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    remote_session_description.sdp(&offer);
    JsFuture::from(peer_connection.set_remote_description(&remote_session_description))
        .await
        .map_err(|err| anyhow!("failed to set remote session description: {:?}", err))?;

    let answer = JsFuture::from(peer_connection.create_answer())
        .await
        .map_err(|err| anyhow!("failed to create SDP answer: {:?}", err))?;
    let answer = Reflect::get(&answer, &JsValue::from_str("sdp"))
        .map_err(|err| {
            anyhow!(
                "failed to get value for 'sdp' key from answer object: {:?}",
                err
            )
        })?
        .as_string()
        .ok_or_else(|| anyhow!("failed to represent object value as string"))?;

    let mut local_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    local_session_description.sdp(&answer);
    JsFuture::from(peer_connection.set_local_description(&local_session_description))
        .await
        .map_err(|err| anyhow!("failed to set local description: {:?}", err))?;

    Ok(answer)
}

pub fn set_peer_connection_on_negotiation_needed(peer_connection: &RtcPeerConnection) {
    let on_negotiation_needed: Box<dyn FnMut()> = Box::new(move || {
        debug!("on negotiation needed event occurred");
    });
    let on_negotiation_needed = Closure::wrap(on_negotiation_needed);
    peer_connection.set_onnegotiationneeded(Some(on_negotiation_needed.as_ref().unchecked_ref()));
    on_negotiation_needed.forget();
}

pub fn set_peer_connection_on_ice_gathering_state_change(peer_connection: &RtcPeerConnection) {
    let peer_connection_clone = peer_connection.clone();
    let on_ice_gathering_state_change: Box<dyn FnMut()> = Box::new(move || {
        debug!(
            "ice gathering state: {:?}",
            peer_connection_clone.ice_gathering_state()
        );
    });
    let on_ice_gathering_state_change = Closure::wrap(on_ice_gathering_state_change);
    peer_connection.set_onicegatheringstatechange(Some(
        on_ice_gathering_state_change.as_ref().unchecked_ref(),
    ));
    on_ice_gathering_state_change.forget();
}

#[cfg(test)]
mod test {
    use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
    use web_sys::{RtcIceConnectionState, RtcIceGatheringState};

    use super::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn test_create_stun_peer_connection_is_successful() {
        let peer_connection = create_peer_connection(&ConnectionType::Local)
            .expect("creating peer connection failed!");
        assert_eq!(
            peer_connection.ice_connection_state(),
            RtcIceConnectionState::New
        );
        assert_eq!(
            peer_connection.ice_gathering_state(),
            RtcIceGatheringState::New
        );
    }

    #[wasm_bindgen_test]
    async fn test_create_sdp_offer_is_successful() {
        let peer_connection = RtcPeerConnection::new().expect("failed to create peer connection");
        let _offer = create_sdp_offer(&peer_connection)
            .await
            .expect("failed to create SDP offer");
        assert!(peer_connection.local_description().is_some());
    }

    #[wasm_bindgen_test]
    async fn test_create_sdp_answer_is_successful() {
        let peer_connection = RtcPeerConnection::new().expect("failed to create peer connection");
        let offer = create_sdp_offer(&peer_connection)
            .await
            .expect("failed to create SDP offer");
        let _answer = create_sdp_answer(&peer_connection, offer)
            .await
            .expect("failed to create SDP answer");
        assert!(peer_connection.local_description().is_some());
        assert!(peer_connection.remote_description().is_some());
    }
}
