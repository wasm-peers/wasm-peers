#![allow(unused_variables)]
use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    MessageEvent, RtcDataChannelEvent, RtcPeerConnection, RtcPeerConnectionIceEvent, RtcSdpType,
    RtcSessionDescriptionInit,
};

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}
macro_rules! console_warn {
    ($($t:tt)*) => (warn(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

pub trait Message {}

struct WebRtcConnection {}

type MessageReceivedCallback = fn();

pub struct Player {
    pub id: String,
    connection: WebRtcConnection,
}

pub struct Client {}

impl Client {
    fn new(message_received_callback: MessageReceivedCallback) -> Self {
        Client {}
    }
}

pub struct MiniServer {}

impl MiniServer {
    fn new(message_received_callback: MessageReceivedCallback) -> Self {
        MiniServer {}
    }
}

pub struct ClientSync<M: Message> {
    messages_queue: Vec<M>,
}

impl<M: Message> ClientSync<M> {
    /// Send a message to mini-server
    pub fn send_message(message: M) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Recieve all messages sent by mini-server
    pub fn recieve_messages() -> Result<Vec<M>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}

pub struct MiniServerSync<M: Message> {
    messages_queue: Vec<M>,
}

impl<M: Message> MiniServerSync<M> {
    /// Send a message to specified player
    pub fn send_message(player: &Player, message: M) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Recieve all messages sent by players since last invocation
    pub fn recieve_messages<T: Message>() -> Result<Vec<M>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}

struct NetworkManager {}

impl NetworkManager {}
