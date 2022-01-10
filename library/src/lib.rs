pub mod common;
mod mini_client;
mod mini_server;
pub mod network_manager;

use log::{debug, info};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console;

use crate::common::set_panic_hook;
use crate::mini_client::MiniClient;
use crate::mini_server::MiniServer;
use crate::network_manager::{ConnectionType, NetworkManager};

// #[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    set_panic_hook();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    debug!("wasm main started");

    let server = NetworkManager::start("TODO-session-id".to_string(), ConnectionType::Local, true)?;
    let client =
        NetworkManager::start("TODO-session-id".to_string(), ConnectionType::Local, false)?;

    for _ in 0..5 {
        match server.borrow().send_message("hello honey, I love you") {
            Ok(_) => debug!("success"),
            Err(error) => debug!("failure: {:?}", error),
        }
    }

    Ok(())
}
