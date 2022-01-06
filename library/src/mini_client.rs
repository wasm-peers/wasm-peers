use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};

use crate::common::create_peer_connection;
use web_sys::{console, RtcDataChannel};
use web_sys::{MessageEvent, RtcDataChannelEvent, RtcPeerConnection, RtcPeerConnectionIceEvent};

pub struct MiniClient {
    peer_connection: RtcPeerConnection,
    data_channel: Option<RtcDataChannel>,
}

impl MiniClient {
    pub fn new() -> Result<Rc<RefCell<Self>>, JsValue> {
        let peer_connection = create_peer_connection()?;

        let client = Rc::new(RefCell::new(MiniClient {
            peer_connection,
            data_channel: None,
        }));

        // set ondatachannel
        {
            let client_clone = client.clone();
            let ondatachannel_closure = Closure::wrap(Box::new(move |ev: RtcDataChannelEvent| {
                let data_channel = ev.channel();
                console::log_1(
                    &format!(
                        "peer_connection_2.ondatachannel: {:?}",
                        data_channel.label()
                    )
                    .into(),
                );

                let onmessage_closure =
                    Closure::wrap(
                        Box::new(move |ev: MessageEvent| match ev.data().as_string() {
                            Some(message) => {
                                console::log_1(
                                    &format!("message to peer connection 2: {:?}", message).into(),
                                );
                            }
                            None => {}
                        }) as Box<dyn FnMut(MessageEvent)>,
                    );
                data_channel.set_onmessage(Some(onmessage_closure.as_ref().unchecked_ref()));
                onmessage_closure.forget();

                client_clone.borrow_mut().data_channel = Some(data_channel);
            })
                as Box<dyn FnMut(RtcDataChannelEvent)>);
            client
                .borrow_mut()
                .peer_connection
                .set_ondatachannel(Some(ondatachannel_closure.as_ref().unchecked_ref()));
            ondatachannel_closure.forget();
        }

        // set onicecandidate
        {
            let onicecandidate_closure = Closure::wrap(Box::new(
                move |ev: RtcPeerConnectionIceEvent| match ev.candidate() {
                    Some(candidate) => {
                        console::log_1(
                            &format!(
                                "peer_connection_2.onicecandidate: {:#?}",
                                candidate.candidate()
                            )
                            .into(),
                        );
                        // TODO: how to get peer_connection_2 here?
                        // peer_connection_1_clone.add_ice_candidate_with_opt_rtc_ice_candidate(Some(&candidate));
                    }
                    None => {}
                },
            )
                as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);
            client
                .borrow_mut()
                .peer_connection
                .set_onicecandidate(Some(onicecandidate_closure.as_ref().unchecked_ref()));
            onicecandidate_closure.forget();
        }

        Ok(client)
    }

    pub(crate) fn get_peer_connection(&self) -> RtcPeerConnection {
        self.peer_connection.clone()
    }
}
