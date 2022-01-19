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
use rusty_games_library::ConnectionType;
use rusty_games_library::one_to_one::NetworkManager;
use web_sys::console;

let session_id = SessionId::new("some-session-id".to_string());
let mut server = NetworkManager::new(
    WS_IP_ADDRESS,
    session_id.clone(),
    ConnectionType::Stun,
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

// #[deny(missing_docs)]

pub mod one_to_many;
pub mod one_to_one;
mod utils;

use std::cell::RefCell;
use std::rc::Rc;
use web_sys::console;
pub use rusty_games_protocol::SessionId;
pub use utils::ConnectionType;
use crate::one_to_many::NetworkManager;
use wasm_bindgen::prelude::wasm_bindgen;

/// Returns a new SessionId instance that can be used to identify a session by signaling server.
pub fn get_random_session_id() -> SessionId {
    SessionId::new(uuid::Uuid::new_v4().to_string())
}

#[wasm_bindgen(start)]
pub fn single_message_passes_both_ways() {
    console_error_panic_hook::set_once();
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));

    const WS_IP_ADDRESS: &str = "ws://0.0.0.0:9001/ws";

    let server_received_message = Rc::new(RefCell::new(false));
    let client_received_message = Rc::new(RefCell::new(false));

    let mut server = NetworkManager::new(
        WS_IP_ADDRESS,
        SessionId::new("dummy-session-id".to_string()),
        ConnectionType::Stun,
        true,
    )
        .unwrap();
    let server_open_connections_count = Rc::new(RefCell::new(0));

    let server_clone = server.clone();
    let server_on_open = {
        let server_open_connections_count = server_open_connections_count.clone();
        move |user_id| {
            console::log_1(&format!("connection to user established: {:?}", user_id).into());
            *server_open_connections_count.borrow_mut() += 1;
            if *server_open_connections_count.borrow() == 2 {
                server_clone.send_message_to_all("ping!");
            }
        }
    };
    let server_on_message = {
        let server_received_message = server_received_message.clone();
        move |user_id, message| {
            console::log_1(
                &format!(
                    "server received message from client {:?}: {}",
                    user_id, message
                )
                    .into(),
            );
            *server_received_message.borrow_mut() = true;
        }
    };
    server.start(server_on_open, server_on_message).unwrap();

    let client_generator = || {
        let mut client = NetworkManager::new(
            WS_IP_ADDRESS,
            SessionId::new("dummy-session-id".to_string()),
            ConnectionType::Stun,
            false,
        )
            .unwrap();
        let client_on_open = |_| { /* do nothing */ };
        let client_clone = client.clone();
        let client_on_message = {
            let client_received_message = client_received_message.clone();
            move |_, message| {
                console::log_1(&format!("client received message: {}", message).into());
                client_clone.send_message_to_all("pong!");
                *client_received_message.borrow_mut() = true;
            }
        };
        client.start(client_on_open, client_on_message).unwrap();
        client
    };
    client_generator();
    client_generator();

    // assert!(*client_received_message.borrow());
    // assert!(*server_received_message.borrow());
}
