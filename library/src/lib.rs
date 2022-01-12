mod mini_client;
mod mini_server;
pub mod network_manager;

use wasm_bindgen::JsValue;
pub use rusty_games_protocol::SessionId;
pub use crate::network_manager::{ConnectionType, NetworkManager};
pub use crate::network_manager::utils::set_panic_hook;

pub fn get_random_string() -> Result<String, wasm_bindgen::JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("Expected window object to be present!"))?;
    let crypto = window.crypto()?;
    let mut array = [0u8; 32];
    crypto.get_random_values_with_u8_array(&mut array)?;
    Ok(array.iter().map(|&x| x as char).collect())
}
