//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use rusty_games_library::many_to_many::NetworkManager;
use rusty_games_library::{ConnectionType, SessionId};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::console;

const WS_IP_ADDRESS: &str = "ws://0.0.0.0:9001/one-to-many";

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn network_manager_starts_successfully() {
    let mut server = NetworkManager::new(
        WS_IP_ADDRESS,
        SessionId::new("dummy-session-id".to_string()),
        ConnectionType::Stun,
    )
    .unwrap();
    server.start(|_| {}, |_, _| {}).unwrap();
}

#[wasm_bindgen_test]
fn single_message_passes_between_all() {
    let opened_connections_count = Rc::new(RefCell::new(0));
    let received_messages_count = Rc::new(RefCell::new(0));

    let peer_generator = || {
        let mut server = NetworkManager::new(
            WS_IP_ADDRESS,
            SessionId::new("dummy-session-id".to_string()),
            ConnectionType::Stun,
        )
        .unwrap();

        let server_clone = server.clone();
        let opened_connections_count = opened_connections_count.clone();
        let server_on_open = {
            move |user_id| {
                console::log_1(&format!("connection to user established: {:?}", user_id).into());
                *opened_connections_count.borrow_mut() += 1;
                server_clone.send_message(user_id, "ping!");
            }
        };

        let server_clone = server.clone();
        let received_messages_count = received_messages_count.clone();
        let server_on_message = {
            move |user_id, message| {
                console::log_1(
                    &format!(
                        "server received message from client {:?}: {}",
                        user_id, message
                    )
                    .into(),
                );
                *received_messages_count.borrow_mut() += 1;
                server_clone.send_message(user_id, "pong!");
            }
        };
        server.start(server_on_open, server_on_message).unwrap();
    };
    peer_generator();
    peer_generator();
    peer_generator();
    peer_generator();

    // assert!(*client_received_message.borrow());
    // assert!(*server_received_message.borrow());
}
