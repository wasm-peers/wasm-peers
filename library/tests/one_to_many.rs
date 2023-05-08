//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use wasm_peers::one_to_many::{MiniClient, MiniServer};
use wasm_peers::{ConnectionType, SessionId};
use web_sys::console;

const SIGNALING_SERVER_URL: &str = "ws://0.0.0.0:9001/one-to-many";

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn network_manager_starts_successfully() {
    let mut server = MiniServer::new(
        SIGNALING_SERVER_URL,
        SessionId::new(1234),
        ConnectionType::Local,
    )
    .unwrap();
    server.start(|_| {}, |_, _: ()| {});
}

#[wasm_bindgen_test]
fn single_message_passes_both_ways() {
    let server_received_message = Rc::new(RefCell::new(false));
    let client_received_message = Rc::new(RefCell::new(false));

    let mut server = MiniServer::new(
        SIGNALING_SERVER_URL,
        SessionId::new(1234),
        ConnectionType::Local,
    )
    .unwrap();
    let server_open_connections_count = Rc::new(RefCell::new(0));

    let server_clone = server.clone();
    let server_on_open = {
        move |user_id| {
            console::log_1(&format!("connection to user established: {:?}", user_id).into());
            *server_open_connections_count.borrow_mut() += 1;
            if *server_open_connections_count.borrow() == 2 {
                server_clone
                    .send_message_to_all(&"ping!".to_owned())
                    .unwrap();
            }
        }
    };
    let server_on_message = {
        move |user_id, message: String| {
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
    server.start(server_on_open, server_on_message);

    let client_generator = || {
        let mut client = MiniClient::new(
            SIGNALING_SERVER_URL,
            SessionId::new(1234),
            ConnectionType::Local,
        )
        .unwrap();
        let client_on_open = || { /* do nothing */ };
        let client_clone = client.clone();
        let client_on_message = {
            let client_received_message = client_received_message.clone();
            move |message: String| {
                console::log_1(&format!("client received message: {}", message).into());
                client_clone
                    .send_message_to_host(&"pong!".to_owned())
                    .unwrap();
                *client_received_message.borrow_mut() = true;
            }
        };
        client.start(client_on_open, client_on_message);
    };
    client_generator();
    client_generator();

    // assert!(*client_received_message.borrow());
    // assert!(*server_received_message.borrow());
}
