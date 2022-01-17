/*!
This crate provides an easy to use wrapper around WebRTC and DataChannels for a peer to peer connections.

# Overview

As creator of agar.io famously stated [WebRTC is hard](https://news.ycombinator.com/item?id=13264952).
This library aims to help, by abstracting away all the setup, and providing a simple way to send
and receive messages over the data channel.

It's as easy as providing address to a signaling server instance and specifying two callbacks.
One for when the connection opens, and one for the messages received.
After that you can send messages back and forth without worrying about the implementation details.

# Example

This example shows two peers sending `ping` and `pong` messages to each other.

```
use rusty_games_library::{ConnectionType, NetworkManager};
use web_sys::console;

let session_id = "some-session-id".to_string();
let mut server = NetworkManager::new(
    WS_IP_ADDRESS,
    session_id.clone(),
    ConnectionType::Stun,
    true,
)
.unwrap();

let server_clone = server.clone();
let server_on_open = move || server_clone.send_message("ping!").unwrap();
let server_on_message = {
    let server_received_message = server_received_message.clone();
    move |message| {
        console::log_1(&format!("server received message: {}", message).into());
        *server_received_message.borrow_mut() = true;
    }
};
server.start(server_on_open, server_on_message).unwrap();

let mut client = NetworkManager::new(
    WS_IP_ADDRESS,
    session_id,
    ConnectionType::Stun,
    false,
)
.unwrap();
let client_on_open = || { /* do nothing */ };
let client_clone = client.clone();
let client_on_message = {
    let client_received_message = client_received_message.clone();
    move |message| {
        console::log_1(&format!("client received message: {}", message).into());
        client_clone.send_message("pong!").unwrap();
        *client_received_message.borrow_mut() = true;
    }
};
client.start(client_on_open, client_on_message).unwrap();
```
*/

#[deny(missing_docs)]

mod network_manager;

pub use crate::network_manager::{ConnectionType, NetworkManager};
pub use rusty_games_protocol::SessionId;

/// Returns a new SessionId instance that can be used to identify a session by signaling server.
pub fn get_random_session_id() -> SessionId {
    uuid::Uuid::new_v4().to_string()
}
