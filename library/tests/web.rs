//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use rusty_games_library::network_manager::{ConnectionType, NetworkManager};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::console;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn dummy_test() {
    assert_eq!(1 + 1, 2);
}

const WS_IP_PORT: &str = "ws://0.0.0.0:9001/ws";

#[wasm_bindgen_test]
fn single_message_passes() -> Result<(), JsValue> {
    // set_panic_hook();
    // wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    // debug!("wasm main started");

    let server = NetworkManager::new(
        WS_IP_PORT,
        "TODO-session-id".to_string(),
        ConnectionType::Stun,
        true,
    );
    let server_clone = server.clone();
    let server_on_open = move || server_clone.send_message("ping!").unwrap();
    let server_on_message =
        |message| console::log_1(&format!("server received message: {}", message).into());
    server.start(server_on_open, server_on_message, true)?;

    let client = NetworkManager::new(
        WS_IP_PORT,
        "TODO-session-id".to_string(),
        ConnectionType::Stun,
        false,
    );
    let client_on_open = || { /* do nothing */ };
    let client_clone = client.clone();
    let client_on_message = |message| {
        console::log_1(&format!("client received message: {}", message).into());
        client_clone.send_message("pong!").unwrap();
    };
    client.start(client_on_open, client_on_message, false)?;

    Ok(())
}
