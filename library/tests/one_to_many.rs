//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use rusty_games_library::one_to_many::{MiniClient, MiniServer};
use rusty_games_library::ConnectionType;
use std::cell::RefCell;
use std::rc::Rc;

use rusty_games_protocol::SessionId;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::console;

const WS_IP_ADDRESS: &str = "ws://0.0.0.0:9001/ws";

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn network_manager_starts_successfully() {
    let mut server = MiniServer::new(
        WS_IP_ADDRESS,
        SessionId::new("dummy-session-id".to_string()),
        ConnectionType::Stun,
    )
    .unwrap();
    server.start(|_| {}, |_, _| {}).unwrap();
}

#[wasm_bindgen_test]
fn single_message_passes_both_ways() {
    let server_received_message = Rc::new(RefCell::new(false));
    let client_received_message = Rc::new(RefCell::new(false));

    let mut server = MiniServer::new(
        WS_IP_ADDRESS,
        SessionId::new("dummy-session-id".to_string()),
        ConnectionType::Stun,
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
        let mut client = MiniClient::new(
            WS_IP_ADDRESS,
            SessionId::new("dummy-session-id".to_string()),
            ConnectionType::Stun,
        )
        .unwrap();
        let client_on_open = |_| { /* do nothing */ };
        let client_clone = client.clone();
        let client_on_message = {
            let client_received_message = client_received_message.clone();
            move |_, message| {
                console::log_1(&format!("client received message: {}", message).into());
                client_clone.send_message_to_host("pong!");
                *client_received_message.borrow_mut() = true;
            }
        };
        client.start(client_on_open, client_on_message).unwrap();
    };
    client_generator();
    client_generator();

    // assert!(*client_received_message.borrow());
    // assert!(*server_received_message.borrow());
}
