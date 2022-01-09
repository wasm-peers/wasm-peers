use log::info;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use rusty_games_library::network_manager::{ConnectionType, NetworkManager};
use rusty_games_library::common::set_panic_hook;

#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    set_panic_hook();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    info!("wasm server started");
    let server = NetworkManager::start("TODO-session-id".to_string(), ConnectionType::Local, true)?;

    Ok(())
}
