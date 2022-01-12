use std::rc::Rc;
use std::sync::RwLock;
use web_sys::{HtmlTextAreaElement};

use wasm_bindgen::JsCast;
use yew::{html, Component, Context, Html, InputEvent, Properties};

use rusty_games_library::network_manager::NetworkManager;
use rusty_games_library::{ConnectionType, SessionId};

pub(crate) enum DocumentMsg {
    UpdateValue,
}

#[derive(Clone, Debug, Eq, PartialEq, Properties)]
pub struct DocumentProps {
    pub session_id: SessionId,
    pub is_host: bool,
}

pub(crate) struct Document {
    network_manager: NetworkManager,
}

impl Component for Document {
    type Message = DocumentMsg;
    type Properties = DocumentProps;

    fn create(ctx: &Context<Self>) -> Self {
        let props = ctx.props();
        let mut network_manager = NetworkManager::new(
            props.session_id.clone(),
            ConnectionType::Stun,
            props.is_host,
        )
        .unwrap();
        let on_message_callback = {
            move |message: String| {
                web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("document-textarea")
                    .expect("could not find textarea element by id")
                    .dyn_ref::<HtmlTextAreaElement>()
                    .expect("element is not a textarea")
                    .set_value(&message);
            }
        };
        network_manager
            .start(|| {}, on_message_callback)
            .expect("couldn't start network manager");
        Self {
            network_manager,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::UpdateValue => {
                let textarea_value = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("document-textarea")
                    .expect("could not find textarea element by id")
                    .dyn_ref::<HtmlTextAreaElement>()
                    .expect("element is not a textarea")
                    .value();
                self.network_manager
                    .send_message(&textarea_value)
                    .expect("cound't send message");
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx
            .link()
            .callback(|e: InputEvent| Self::Message::UpdateValue);
        html! {
            <textarea id={ "document-textarea" } { oninput }/>
        }
    }
}
