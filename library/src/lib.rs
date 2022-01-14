mod mini_client;
mod mini_server;
mod network_manager;

pub use crate::network_manager::{ConnectionType, NetworkManager};
pub use rusty_games_protocol::SessionId;

pub fn get_random_session_id() -> SessionId {
    uuid::Uuid::new_v4().to_string()
}
