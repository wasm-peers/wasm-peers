// use crate::network_manager::NetworkManager;
// use crate::ConnectionType;
// use rusty_games_protocol::SessionId;
// use wasm_bindgen::JsValue;
//
// pub struct MiniServer {
//     network_manager: NetworkManager,
// }
//
// impl MiniServer {
//     pub fn start(session_id: SessionId, connection_type: ConnectionType) -> Result<Self, JsValue> {
//         let network_manager = NetworkManager::start(session_id, connection_type, true)?;
//
//         Ok(MiniServer { network_manager })
//     }
// }
