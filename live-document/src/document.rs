use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::{console, HtmlTextAreaElement, UrlSearchParams};
use yew::{html, Component, Context, Html};

use rusty_games_library::one_to_many::{MiniClient, MiniServer};
use rusty_games_library::{ConnectionType, SessionId};

pub(crate) enum DocumentMsg {
    UpdateValue,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentQuery {
    pub session_id: String,
    pub is_host: bool,
}

impl DocumentQuery {
    pub(crate) fn new(session_id: String, is_host: bool) -> Self {
        DocumentQuery {
            session_id,
            is_host,
        }
    }
}

enum Role {
    Host(MiniServer),
    Client(MiniClient),
}

pub(crate) struct Document {
    session_id: SessionId,
    role: Role,
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
        let session_id = SessionId::new(query_params.get("session_id").unwrap());
        let is_host = query_params.get("is_host").unwrap() == "true";

        let is_ready = Rc::new(RefCell::new(false));
        let role = if is_host {
            let mut mini_server =
                MiniServer::new(env!("WS_IP_PORT"), session_id.clone(), ConnectionType::Stun)
                    .unwrap();
            let on_open_callback = {
                let mini_server = mini_server.clone();
                let is_ready = is_ready.clone();
                move |user_id| {
                    if !*is_ready.borrow() {
                        get_text_area().set_disabled(false);
                        *is_ready.borrow_mut() = true;
                    }
                    let value = get_text_area().value();
                    mini_server
                        .send_message(user_id, &value)
                        .expect("failed to send current input to new connection");
                }
            };
            let on_message_callback = {
                let mini_server = mini_server.clone();
                move |_, message: String| {
                    get_text_area().set_value(&message);
                    mini_server.send_message_to_all(&message);
                }
            };
            mini_server
                .start(on_open_callback, on_message_callback)
                .expect("mini server failed to start");
            Role::Host(mini_server)
        } else {
            let mut mini_client =
                MiniClient::new(env!("WS_IP_PORT"), session_id.clone(), ConnectionType::Stun)
                    .unwrap();
            let on_open_callback = {
                let is_ready = is_ready.clone();
                move |_| {
                    if !*is_ready.borrow() {
                        get_text_area().set_disabled(false);
                        *is_ready.borrow_mut() = true;
                    }
                }
            };
            let on_message_callback = {
                move |_, message: String| {
                    get_text_area().set_value(&message);
                }
            };
            mini_client
                .start(on_open_callback, on_message_callback)
                .expect("mini client failed to start");
            Role::Client(mini_client)
        };
        Self {
            is_ready,
            session_id,
            role,
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Self::Message::UpdateValue => {
                let textarea_value = get_text_area().value();
                match &self.role {
                    Role::Host(mini_server) => {
                        mini_server.send_message_to_all(&textarea_value);
                    }
                    Role::Client(mini_client) => {
                        mini_client
                            .send_message_to_host(&textarea_value)
                            .unwrap_or_else(|_| {
                                console::error_1(&"couldn't send a message!".to_string().into())
                            });
                    }
                }
            }
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let oninput = ctx.link().callback(|_| Self::Message::UpdateValue);
        let disabled = !*self.is_ready.borrow();
        let placeholder = if disabled {
            "This is a live document shared with a different user.\nYou will be allowed to write once other user connects."
        } else {
            "This is a live document shared with a different user.\nWhat you both write will be visible to the other."
        };
        html! {
            <main>
                <p> { "Session id: " } { &self.session_id.inner } </p>
                <textarea id={ "document-textarea" } { disabled } { placeholder } { oninput }/>
            </main>
        }
    }
}
