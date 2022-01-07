mod common;
mod mini_client;
mod mini_server;
mod network_manager;

use log::{debug, info};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console;

use crate::common::set_panic_hook;
use crate::mini_client::MiniClient;
use crate::mini_server::MiniServer;
use crate::network_manager::NetworkManager;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = isServer)]
    fn is_server() -> bool;
    #[wasm_bindgen(js_namespace = getHash)]
    fn get_hash() -> String;
}

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    set_panic_hook();

    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    debug!("wasm main started");

    let server = NetworkManager::start("TODO-session-id".to_string())?;
    let client = NetworkManager::start("TODO-session-id".to_string())?;

    // server
    //     .borrow()
    //     .send_message("channel is open and send_message works")?;

    Ok(())
}
