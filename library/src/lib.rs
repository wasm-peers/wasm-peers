mod mini_client;
mod mini_server;
pub mod network_manager;

pub use crate::network_manager::utils::set_panic_hook;
pub use crate::network_manager::{ConnectionType, NetworkManager};
pub use rusty_games_protocol::SessionId;
use uuid::Uuid;

pub fn get_random_session_id() -> SessionId {
    Uuid::new_v4().to_string()
}
