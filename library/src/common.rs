use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use log::debug;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, RtcConfiguration, RtcPeerConnection};
use web_sys::{RtcSdpType, RtcSessionDescriptionInit};

use crate::mini_client::MiniClient;
use crate::mini_server::MiniServer;

const STUN_SERVER: &str = "stun:stun.l.google.com:19302";

pub const WS_IP_PORT: &str = "ws://0.0.0.0:9001/ws";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_m_id: String,
    pub sdp_m_line_index: u16,
}

pub(crate) fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

pub(crate) fn create_peer_connection() -> Result<RtcPeerConnection, JsValue> {
    let ice_servers = Array::new();
    {
        let server_entry = Object::new();

        Reflect::set(&server_entry, &"urls".into(), &STUN_SERVER.into())?;

        ice_servers.push(&*server_entry);
    }

    let mut rtc_configuration = RtcConfiguration::new();
    rtc_configuration.ice_servers(&ice_servers);

    RtcPeerConnection::new_with_configuration(&rtc_configuration)
}

pub(crate) async fn create_sdp_offer(
    peer_connection: RtcPeerConnection,
) -> Result<String, JsValue> {
    let offer = JsFuture::from(peer_connection.create_offer()).await?;
    let offer = Reflect::get(&offer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap();
    let mut local_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    local_session_description.sdp(&offer);
    JsFuture::from(peer_connection.set_local_description(&local_session_description)).await?;

    Ok(offer)
}

pub(crate) async fn create_sdp_answer(
    peer_connection: RtcPeerConnection,
    offer: String,
) -> Result<String, JsValue> {
    debug!("create_sdp_answer");
    let mut remote_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    remote_session_description.sdp(&offer);
    let promise = peer_connection.set_remote_description(&remote_session_description);
    // FIXME: this promise raises an exception
    JsFuture::from(promise).await?;
    debug!("set remote description successfully");

    let promise = peer_connection.create_answer();
    // FIXME: this promise raises an exception too
    let answer = JsFuture::from(promise).await?;
    let answer = Reflect::get(&answer, &JsValue::from_str("sdp"))?
        .as_string()
        .unwrap();
    debug!("created answer successfully: {}", answer);

    let mut local_session_description = RtcSessionDescriptionInit::new(RtcSdpType::Answer);
    local_session_description.sdp(&answer);
    JsFuture::from(peer_connection.set_local_description(&local_session_description)).await?;

    Ok(answer)
}
