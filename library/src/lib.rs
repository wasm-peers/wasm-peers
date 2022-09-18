/*!
This crate provides an easy-to-use wrapper around `WebRTC` and `DataChannels` for a peer to peer connections.

# Overview

As creator of [agar.io](https://agar.io) famously stated [`WebRTC` is hard](https://news.ycombinator.com/item?id=13264952).
This library aims to help, by abstracting away all the setup, and providing a simple way to send
and receive messages over the data channel.

It's as easy as providing address to a signaling server instance from
[accompanying crate](https://docs.rs/wasm-peers-signaling-server/latest/wasm_peers_signaling_server/) and specifying two callbacks.
One for when a connection opens, and one for when a message is received.
After that you can send messages back and forth without worrying about the implementation details.

Library contains three network , [one-to-one](one_to_one), which creates an equal connection between two peers,
[one-to-many](one_to_many), which specifies a host and arbitrary number of clients
and [many-to-many] that creates connection for each pair of peers and allows sending messages to any of them.

*/

#[deny(missing_docs)]
#[warn(clippy::pedantic)]
#[cfg(feature = "many-to-many")]
pub mod many_to_many;
#[cfg(feature = "one-to-many")]
pub mod one_to_many;
#[cfg(feature = "one-to-one")]
pub mod one_to_one;
mod utils;

pub use utils::ConnectionType;
pub use wasm_peers_protocol::{SessionId, UserId};

/// Returns a new `SessionId` instance that can be used to identify a session by signaling server.
pub fn get_random_session_id() -> SessionId {
    SessionId::new(uuid::Uuid::new_v4().to_string())
}
