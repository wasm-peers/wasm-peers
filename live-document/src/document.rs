use rusty_games_library::many_to_many::NetworkManager;
use rusty_games_library::{get_random_session_id, ConnectionType, SessionId};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{HtmlTextAreaElement, UrlSearchParams};
use yew::{html, Component, Context, Html};

pub(crate) enum DocumentMsg {
    UpdateValue,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentQuery {
    pub session_id: String,
}

impl DocumentQuery {
    pub(crate) fn new(session_id: String) -> Self {
        DocumentQuery { session_id }
    }
}

pub(crate) struct Document {
    session_id: SessionId,
    network_manager: NetworkManager,
    is_ready: Rc<RefCell<bool>>,
}

fn get_query_params() -> UrlSearchParams {
    let search = web_sys::window().unwrap().location().search().unwrap();
    UrlSearchParams::new_with_str(&search).unwrap()
}

fn get_text_area() -> HtmlTextAreaElement {
    web_sys::window()
        .unwrap()
        .document()
        .expect("document node is missing")
        .get_element_by_id("document-textarea")
        .expect("could not find textarea element by id")
        .dyn_into::<HtmlTextAreaElement>()
        .expect("element is not a textarea")
}

impl Component for Document {
    type Message = DocumentMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let query_params = get_query_params();
        let session_id = match query_params.get("session_id") {
            Some(session_string) => SessionId::new(session_string),
            None => {
                let location = web_sys::window().unwrap().location();
                let generated_session_id = get_random_session_id();
                query_params.append("session_id", generated_session_id.as_str());
                let search: String = query_params.to_string().into();
                location.set_search(&search).unwrap();
                generated_session_id
            }
        };

        let is_ready = Rc::new(RefCell::new(false));
        let mut network_manager =
            NetworkManager::new(env!("WS_IP_PORT"), session_id.clone(), ConnectionType::Stun)
                .unwrap();
        let on_open_callback = {
            let mini_server = network_manager.clone();
            let is_ready = is_ready.clone();
            move |user_id| {
                if !*is_ready.borrow() {
                    get_text_area().set_disabled(false);
                    get_text_area().set_placeholder("This is a live document shared with a other users.\nWhat you write will be visible to all.");
                    *is_ready.borrow_mut() = true;
                }
                let value = get_text_area().value();
                if !value.is_empty() {
                    mini_server
                        .send_message(user_id, &value)
                        .expect("failed to send current input to new connection");
                }
            }
        };
        let on_message_callback = {
            move |_, message: String| {
                get_text_area().set_value(&message);
            }
        };
        network_manager
            .start(on_open_callback, on_message_callback)
            .expect("mini server failed to start");
        Self {
            is_ready,
            network_manager,
            session_id,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::UpdateValue => {
                let textarea_value = get_text_area().value();
                self.network_manager.send_message_to_all(&textarea_value);
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().callback(|_| Self::Message::UpdateValue);
        let disabled = !*self.is_ready.borrow();
        let placeholder = "This is a live document shared with other users.\nYou will be allowed to write once other join, or your connection is established.";
        html! {
            <main style="text-align:center">
                <p> { "Share session id: " } { &self.session_id } </p>
                <p> { "or just copy the page url." } </p>
                <textarea id={ "document-textarea" } cols="100" rows="40" { disabled } { placeholder } { oninput }/>
            </main>
        }
    }
}
