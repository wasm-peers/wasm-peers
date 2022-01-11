pub mod common;
mod mini_client;
mod mini_server;
pub mod network_manager;

use log::{debug, info};

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::common::set_panic_hook;

use crate::network_manager::{ConnectionType, NetworkManager};

#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    set_panic_hook();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    debug!("wasm main started");

    let mut server = NetworkManager::new("TODO-session-id".to_string(), ConnectionType::Stun)?;
    let mut client = NetworkManager::new("TODO-session-id".to_string(), ConnectionType::Stun)?;

    let server_clone = server.clone();
    let server_on_open = move || server_clone.send_message("ping!").unwrap();
    let server_on_message = |message| info!("server received message: {}", &message);
    let client_on_open = || {};
    let client_clone = client.clone();
    let client_on_message = move |message| {
        info!("client received message: {}", &message);
        client_clone.send_message("pong!").unwrap()
    };

    server.start(server_on_open, server_on_message, true)?;
    client.start(client_on_open, client_on_message, false)?;

    Ok(())
}
