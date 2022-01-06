mod client;
mod common;
mod mini_server;

use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console;

use crate::client::Client;
use crate::common::set_panic_hook;
use crate::mini_server::Server;

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

    console::log_1(&"wasm main started".into());

    let server = Server::start("TODO-session-id".to_string())?;
    let client = Client::new()?;

    // server
    //     .borrow()
    //     .send_message("channel is open and send_message works")?;

    Ok(())
}
