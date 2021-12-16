#![allow(unused_variables)]
use std::{cell::RefCell, rc::Rc};

use crate::network_manager::{ConnectionState, NetworkManager, State};

pub trait Message {}

struct WebRtcConnection {}

type MessageReceivedCallback<M> = Box<dyn FnOnce(M)>;

pub struct Player {
    pub id: String,
    connection: WebRtcConnection,
}

pub struct Client<M: Message> {
    network_manager: Rc<RefCell<NetworkManager>>,
    message_received_callback: MessageReceivedCallback<M>,
}

impl<M: Message> Client<M> {
    pub fn new(
        offer_string: String,
        message_received_callback: MessageReceivedCallback<M>,
    ) -> Self {
        let network_manager = Rc::new(RefCell::new(NetworkManager::new(State::Client(
            ConnectionState::default(),
        ))));
        NetworkManager::start_web_rtc(network_manager.clone());
        Client {
            network_manager,
            message_received_callback,
        }
    }
}

pub struct MiniServer<M: Message> {
    network_manager: Rc<RefCell<NetworkManager>>,
    message_received_callback: MessageReceivedCallback<M>,
}

impl<M: Message> MiniServer<M> {
    pub fn new(
        offer_string: String,
        message_received_callback: MessageReceivedCallback<M>,
    ) -> Self {
        let network_manager = Rc::new(RefCell::new(NetworkManager::new(State::Server(
            ConnectionState::default(),
        ))));
        NetworkManager::start_web_rtc(network_manager.clone());
        MiniServer {
            network_manager,
            message_received_callback,
        }
    }
}

pub struct ClientSync<M: Message> {
    messages_queue: Vec<M>,
}

impl<M: Message> ClientSync<M> {
    /// Send a message to mini-server
    pub fn send_message(message: M) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Recieve all messages sent by mini-server
    pub fn recieve_messages() -> Result<Vec<M>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}

pub struct MiniServerSync<M: Message> {
    messages_queue: Vec<M>,
}

impl<M: Message> MiniServerSync<M> {
    /// Send a message to specified player
    pub fn send_message(player: &Player, message: M) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Recieve all messages sent by players since last invocation
    pub fn recieve_messages<T: Message>() -> Result<Vec<M>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}
