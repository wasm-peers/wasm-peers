use wasm_bindgen::prelude::*;

pub mod client_server;
pub mod network_manager;

pub use client_server::{Client, ClientSync, Message, MiniServer, MiniServerSync};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = isServer)]
    fn is_server() -> bool;
    #[wasm_bindgen(js_namespace = getHash)]
    fn get_hash() -> String;
}

fn dummy_callback<M: Message>(message: M) {}

struct DummyMessage {}

impl Message for DummyMessage {}

#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    let hash = get_hash();
    if is_server() {
        MiniServer::<DummyMessage>::new(hash, Box::new(dummy_callback));
    } else {
        Client::<DummyMessage>::new(hash, Box::new(dummy_callback));
    }
    Ok(())
}
