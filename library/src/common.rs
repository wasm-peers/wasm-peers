use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, RtcConfiguration, RtcPeerConnection};
use web_sys::{RtcSdpType, RtcSessionDescriptionInit};

use crate::server::Server;
use crate::Client;

const STUN_SERVER: &str = "stun:stun.l.google.com:19302";

pub const WS_IP_PORT: &str = "ws://0.0.0.0:9001/ws";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IceCandidate {
    candidate: String,
    sdp_mid: String,
    sdp_m_line_index: u16,
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

pub(crate) async fn create_sdp_offer(peer_connection: RtcPeerConnection) -> Result<String, JsValue> {
    let offer = JsFuture::from(peer_connection.create_offer()).await?;
    let offer = Reflect::get(&offer, &JsValue::from_str("sdp"))?.as_string().unwrap();
    let mut session_description = RtcSessionDescriptionInit::new(RtcSdpType::Offer);
    session_description.sdp(&offer);

    JsFuture::from(peer_connection.set_local_description(&session_description)).await?;

    Ok(offer)
}